use crate::adb;
use crate::logger;
use crate::network;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::RwLock;

/// Nombre d'échecs consécutifs requis pour basculer sur le secondaire.
pub const FAILOVER_THRESHOLD: u32 = 2;
/// Nombre de succès consécutifs requis pour restaurer le primaire.
pub const RECOVERY_THRESHOLD: u32 = 3;
/// Nombre d'échecs consécutifs avant d'afficher « Hors ligne ».
///
/// Distinct de `FAILOVER_THRESHOLD`: la bascule concerne le primaire, cet
/// état concerne la connectivité affichée. Un ping perdu isolé (Wi-Fi,
/// changement de route pendant une bascule) ne doit pas faire clignoter
/// l'en-tête toutes les 5 s.
const OFFLINE_THRESHOLD: u32 = 2;

/// Bornes de validation des réglages.
const MIN_INTERVAL_SECS: u64 = 1;
const MAX_INTERVAL_SECS: u64 = 300;
const MIN_REFRESH_SECS: u64 = 5;
const MAX_REFRESH_SECS: u64 = 600;

/// Longueur maximale du chemin `adb.exe` enregistré.
const MAX_ADB_PATH_LEN: usize = 260;

/// Délai minimal entre deux tentatives de partage USB via ADB.
const USB_TETHER_CHECK_SECS: u64 = 10;

// ─── Settings ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub interval_secs: u64,
    pub ping_target: String,
    pub adapter_refresh_secs: u64,
    /// Partage USB du téléphone activé seul au branchement (via ADB).
    ///
    /// `#[serde(default)]` avec un défaut `true`: un `settings.json` écrit
    /// avant cette option doit conserver le comportement attendu.
    #[serde(default = "default_enabled")]
    pub usb_tether_auto: bool,
    /// Chemin de `adb.exe`. Vide: détection automatique (PATH, winget, SDK).
    #[serde(default)]
    pub adb_path: String,
}

/// Valeur par défaut d'un booléen activé (les anciens fichiers n'ont pas le champ).
fn default_enabled() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_secs: 5,
            ping_target: network::DEFAULT_PING_TARGET.to_string(),
            adapter_refresh_secs: 30,
            usb_tether_auto: true,
            adb_path: String::new(),
        }
    }
}

impl Settings {
    fn path() -> std::path::PathBuf {
        let mut p = dirs().unwrap_or_default();
        p.push("settings.json");
        p
    }

    pub fn load() -> Self {
        let path = Self::path();
        let loaded = std::fs::read_to_string(&path)
            .ok()
            .and_then(|data| serde_json::from_str::<Settings>(&data).ok())
            .unwrap_or_default();

        // Un fichier `settings.json` écrit à la main ou par une version
        // antérieure peut contenir des valeurs hors bornes: on applique le
        // même clamp que `save_settings`, sinon le monitor bouclerait trop
        // vite (interval 0) ou attendrait une heure.
        let validated = loaded.clone().validated();

        if loaded.interval_secs != validated.interval_secs
            || loaded.adapter_refresh_secs != validated.adapter_refresh_secs
            || loaded.ping_target != validated.ping_target
        {
            logger::log_file(&format!(
                "Settings clamped au chargement: {:?} -> {:?}",
                loaded, validated
            ));
        }

        validated
    }

    /// Ramène les champs dans leurs bornes admissibles.
    pub fn validated(self) -> Self {
        let ping_target = self.ping_target.trim();
        let ping_target = if ping_target.is_empty() || !network::is_valid_ping_target(ping_target) {
            network::DEFAULT_PING_TARGET.to_string()
        } else {
            ping_target.to_string()
        };

        Self {
            interval_secs: self
                .interval_secs
                .clamp(MIN_INTERVAL_SECS, MAX_INTERVAL_SECS),
            ping_target,
            adapter_refresh_secs: self
                .adapter_refresh_secs
                .clamp(MIN_REFRESH_SECS, MAX_REFRESH_SECS),
            usb_tether_auto: self.usb_tether_auto,
            adb_path: self
                .adb_path
                .trim()
                .chars()
                .take(MAX_ADB_PATH_LEN)
                .collect(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        write_atomic(&path, &json).map_err(|e| e.to_string())
    }
}

/// Écrit `data` dans `path` sans laisser de fichier à moitié écrit.
///
/// `std::fs::write` tronque puis écrit: un arrêt en cours d'écriture laisse
/// un JSON incomplet, et `Settings::load` retombe alors silencieusement sur
/// les défauts (réglages perdus). Écrire à côté puis renommer rend la
/// sauvegarde atomique: `MoveFileEx` remplace la cible sous Windows.
fn write_atomic(path: &std::path::Path, data: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)
}

fn dirs() -> Option<std::path::PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(|base| {
        let mut p = std::path::PathBuf::from(base);
        p.push("DualLink");
        p
    })
}

fn default_status_known() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStatus {
    pub name: String,
    pub reachable: bool,
    pub latency_ms: Option<u64>,
    /// Faux quand la sonde est dégradée (repli `ping -S`): on ne sait pas
    /// si l'adaptateur est réellement joignable. `#[serde(default)]` avec
    /// défaut `true` pour rester lisible par un frontend qui ignore le champ.
    #[serde(default = "default_status_known")]
    pub status_known: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub enabled: bool,
    pub primary_index: u32,
    pub secondary_index: u32,
    pub primary_name: String,
    pub secondary_name: String,
    /// Passerelle du primaire: nécessaire pour l'épingler comme next hop de
    /// la route hôte de test. Absente => failover refusée à l'activation.
    #[serde(default)]
    pub primary_gateway: Option<String>,
    /// Adresse IPv4 du primaire, utilisée comme source des sondes.
    #[serde(default)]
    pub primary_ip: Option<String>,
    /// Cible de sonde résolue en IPv4 à l'activation.
    ///
    /// La route hôte de test exige une adresse numérique: un nom d'hôte
    /// échouerait à chaque tick. Vide dans une configuration héritée, auquel
    /// cas la cible du tick est utilisée.
    #[serde(default)]
    pub probe_target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverState {
    pub config: FailoverConfig,
    pub is_failed_over: bool,
    pub failover_count: u32,
    pub last_switch: String,
}

impl Default for FailoverState {
    fn default() -> Self {
        Self {
            config: FailoverConfig::default(),
            is_failed_over: false,
            failover_count: 0,
            last_switch: "Never".to_string(),
        }
    }
}

// ─── Logique de décision (fonction pure) ─────────────────────────

/// Action à exécuter après une mesure de joignabilité du primaire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverAction {
    None,
    SwitchToSecondary,
    RestorePrimary,
}

/// Compteurs de mesures consécutives, remis à zéro à chaque changement d'état.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FailoverCounters {
    pub failures: u32,
    pub successes: u32,
}

impl FailoverCounters {
    pub fn reset(&mut self) {
        self.failures = 0;
        self.successes = 0;
    }
}

//     ping   failures  affiché
//     ok     0         en ligne
//     échec  1         en ligne   (échec isolé ignoré)
//     échec  2         hors ligne
//
/// État de connectivité affiché, avec hystérésis.
///
/// `known` reste faux tant qu'aucune mesure n'a permis de trancher: l'UI
/// affiche alors « Vérification… » au lieu d'un « Déconnecté » mensonger au
/// démarrage. Un succès efface les échecs accumulés; il faut
/// `OFFLINE_THRESHOLD` échecs consécutifs pour passer hors ligne.
#[derive(Debug, Default, Clone, Copy)]
pub struct Reachability {
    online: bool,
    known: bool,
    failures: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachabilityChange {
    None,
    Online,
    Offline,
}

impl Reachability {
    pub fn observe(&mut self, reachable: bool) -> ReachabilityChange {
        if reachable {
            // Inconnu -> en ligne compte aussi comme un changement, pour
            // journaliser une seule fois le premier état établi.
            let changed = !self.known || !self.online;
            self.online = true;
            self.known = true;
            self.failures = 0;
            return if changed {
                ReachabilityChange::Online
            } else {
                ReachabilityChange::None
            };
        }

        self.failures += 1;
        if self.failures < OFFLINE_THRESHOLD {
            return ReachabilityChange::None;
        }

        let changed = !self.known || self.online;
        self.online = false;
        self.known = true;
        if changed {
            ReachabilityChange::Offline
        } else {
            ReachabilityChange::None
        }
    }

    pub fn is_online(&self) -> bool {
        self.online
    }

    pub fn is_known(&self) -> bool {
        self.known
    }
}

//     Joignable ?  failures  successes  is_failed_over  Action
//     non          1         0           false           None
//     non          2         0           false           Switch
//     oui          0         1           true            None
//     oui          0         3           true            Restore
//
/// Décide de l'action de failover à partir de l'état et de la dernière mesure.
///
/// Fonction pure: aucun accès réseau, aucune horloge. Le hysteresis est
/// asymétrique — 2 échecs pour basculer, 3 succès pour restaurer — ce qui
/// interdit l'oscillation lorsque la joignabilité alterne échec/succès.
///
/// Les compteurs sont remis à zéro au moment où l'action est décidée: si
/// l'application des métriques échoue ensuite, l'appelant ne change pas
/// `is_failed_over` et la prochaine mesure repart de zéro.
///
pub fn decide(
    state: &FailoverState,
    counters: &mut FailoverCounters,
    reachable: bool,
) -> FailoverAction {
    if !state.config.enabled {
        counters.reset();
        return FailoverAction::None;
    }

    if reachable {
        counters.successes += 1;
        counters.failures = 0;

        if state.is_failed_over && counters.successes >= RECOVERY_THRESHOLD {
            counters.reset();
            return FailoverAction::RestorePrimary;
        }
    } else {
        counters.failures += 1;
        counters.successes = 0;

        if !state.is_failed_over && counters.failures >= FAILOVER_THRESHOLD {
            counters.reset();
            return FailoverAction::SwitchToSecondary;
        }
    }

    FailoverAction::None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorState {
    pub internet_reachable: bool,
    /// Faux tant qu'aucune mesure n'a permis de trancher: le frontend affiche
    /// « Vérification… » plutôt qu'un « Déconnecté » au démarrage.
    #[serde(default)]
    pub internet_known: bool,
    pub overall_latency_ms: Option<u64>,
    pub adapters: Vec<AdapterStatus>,
    /// Noms de toutes les cartes vues par le monitor, y compris celles qui ne
    /// sont pas connectées: le frontend compare cette liste à celle qu'il
    /// affiche et recharge les cartes quand le matériel change (téléphone
    /// branché, carte désactivée dans Windows, etc.).
    #[serde(default)]
    pub adapter_names: Vec<String>,
    pub last_check: String,
    pub failover: FailoverState,
}

impl Default for MonitorState {
    fn default() -> Self {
        Self {
            internet_reachable: false,
            internet_known: false,
            overall_latency_ms: None,
            adapters: Vec::new(),
            adapter_names: Vec::new(),
            last_check: "Never".to_string(),
            failover: FailoverState::default(),
        }
    }
}

// ─── Sonde du primaire ────────────────────────────────────────────

/// Évite de répéter le WARN « passerelle inconnue » à chaque tick: le log
/// partait en boucle (12 lignes/s) tant que la passerelle restait absente.
/// Le flag est réarmé dès que la passerelle redevient connue.
static WARN_GATEWAY_MISSING: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

/// Cible réellement épinglée: l'IPv4 résolue à l'activation, sinon la cible
/// du tick (configuration héritée, antérieure à ce champ).
fn probe_host<'a>(config: &'a FailoverConfig, fallback: &'a str) -> &'a str {
    if config.probe_target.trim().is_empty() {
        fallback
    } else {
        &config.probe_target
    }
}

/// Teste la joignabilité du PRIMAIRE via une route hôte épinglée.
///
/// Retourne `None` quand la sonde n'a pas lieu d'être: failover désactivé, ou
/// primaire sans passerelle connue (impossible d'épingler la route). Dans ce
/// cas l'appelant ne prend aucune décision, plutôt que de décider sur une
/// mesure qui ne teste pas le primaire.
///
/// La cible est l'IPv4 résolue à l'activation (`probe_target`): une route
/// `/32` ne peut pas désigner un nom d'hôte.
fn probe_primary(target: &str, config: &FailoverConfig) -> Option<bool> {
    if !config.enabled {
        return None;
    }

    let target = probe_host(config, target);
    let gateway = config.primary_gateway.as_deref()?;
    if gateway.trim().is_empty() || gateway == "N/A" {
        if !WARN_GATEWAY_MISSING.swap(true, std::sync::atomic::Ordering::Relaxed) {
            logger::log_file("WARN sonde primaire: passerelle inconnue, décision suspendue");
        }
        return None;
    }
    WARN_GATEWAY_MISSING.store(false, std::sync::atomic::Ordering::Relaxed);

    let route = network::RouteTarget {
        interface_index: config.primary_index,
        source_ip: config.primary_ip.clone(),
        gateway: Some(gateway.to_string()),
    };

    match network::ping_via_interface(target, &route) {
        Ok(res) => Some(res.reachable),
        Err(e) => {
            // Route impossible à poser: le primaire est considéré injoignable,
            // ce qui conduit à une bascule après FAILOVER_THRESHOLD ticks.
            logger::log_file(&format!(
                "ERROR sonde primaire (if {}): {:#}",
                config.primary_index, e
            ));
            Some(false)
        }
    }
}

// ─── Boucle de monitoring ─────────────────────────────────────────

pub fn start_monitor(
    app_handle: tauri::AppHandle,
    state: Arc<RwLock<MonitorState>>,
    settings: Arc<RwLock<Settings>>,
) {
    logger::log_file("Monitor started");
    tauri::async_runtime::spawn(async move {
        use tokio::time::Duration;

        let mut tick_count: u64 = 0;
        let mut cached_adapters: Vec<network::NetworkAdapter> = Vec::new();
        let mut cached_statuses: Vec<AdapterStatus> = Vec::new();
        let mut counters = FailoverCounters::default();
        let mut reachability = Reachability::default();
        let mut last_failover_enabled = false;
        let mut last_adapter_signature: Option<String> = None;
        let mut last_rndis: Option<String> = None;
        let mut force_adapter_refresh = false;
        let mut last_usb_check = std::time::Instant::now();

        // Partage USB (ADB): le travail se fait hors du cycle de monitoring et
        // rend son résultat par un drapeau lu au tick suivant.
        let adb_busy = Arc::new(AtomicBool::new(false));
        let usb_armed = Arc::new(AtomicBool::new(false));
        let usb_serial: Arc<std::sync::Mutex<Option<String>>> =
            Arc::new(std::sync::Mutex::new(None));
        let usb_warning: Arc<std::sync::Mutex<Option<String>>> =
            Arc::new(std::sync::Mutex::new(None));

        loop {
            tick_count += 1;

            // Résultat du partage USB obtenu au tick précédent: la carte RNDIS
            // arrive, on le signale et on relit la liste des interfaces.
            if usb_armed.swap(false, Ordering::Relaxed) {
                let serial = usb_serial
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take()
                    .unwrap_or_default();
                logger::log_file(&format!("USB: partage activé sur '{serial}'"));
                let _ = app_handle.emit("usb-tethering-armed", &serial);
                force_adapter_refresh = true;
            }

            let Settings {
                interval_secs,
                ping_target,
                adapter_refresh_secs,
                usb_tether_auto,
                adb_path,
            } = settings.read().await.clone().validated();

            let refresh_interval = (adapter_refresh_secs / interval_secs).max(1);

            // ── Failover: état lu AVANT les mesures ──
            // La sonde du primaire a besoin de sa passerelle pour épingler la
            // route: on lit donc la configuration avant de lancer les I/O.
            let failover_cfg = state.read().await.failover.config.clone();

            // ── Mesures réseau (bloquantes: PowerShell + ping.exe) ──
            // Exécutées dans un thread dédié pour ne pas bloquer le runtime
            // async de Tauri (commandes et événements partagent ce runtime).
            let adapters_fresh = force_adapter_refresh
                || tick_count % refresh_interval == 1
                || cached_adapters.is_empty();
            force_adapter_refresh = false;
            let network_tick = tokio::task::spawn_blocking({
                let target = ping_target.clone();
                move || {
                    let adapters = if adapters_fresh {
                        match network::list_adapters() {
                            Ok(list) => Some(list),
                            Err(e) => {
                                logger::log_file(&format!("Monitor list_adapters ERROR: {e:#}"));
                                None
                            }
                        }
                    } else {
                        None
                    };

                    // Statut affiché: ce que voit l'utilisateur, donc la route
                    // par défaut. Pendant un failover c'est le secondaire qui
                    // répond, et c'est bien l'information pertinente.
                    let global = match network::test_connectivity_to(&target) {
                        Ok(res) => res,
                        Err(e) => {
                            logger::log_file(&format!("Monitor ping ERROR ({target}): {e:#}"));
                            network::ConnectivityResult {
                                reachable: false,
                                latency_ms: None,
                                degraded: false,
                            }
                        }
                    };

                    // Décision de failover: on teste le PRIMAIRE, pas la route
                    // par défaut. Un ping global reviendrait à interroger le
                    // secondaire (métrique 100 → 10 pendant un failover), qui
                    // répondrait 3 fois et ferait restaurer un primaire toujours
                    // coupé: c'est l'oscillation que la route épinglée supprime.
                    let primary = probe_primary(&target, &failover_cfg);

                    // Statut par adaptateur: uniquement au rythme de
                    // rafraîchissement. Sonder chaque adaptateur coûte 3
                    // processus (route + ping + retrait), soit ~16 processus
                    // pour 4 adaptateurs: à chaque tick cela effondre le budget
                    // de processus et allonge le tick d'environ 2 s. Un statut
                    // par adaptateur rafraîchi toutes les 30 s est suffisant:
                    // il sert à l'affichage, pas à la décision de failover.
                    // Sans élévation, la route épinglée est impossible et le
                    // repli `ping -S` ne garantit pas l'interface: mieux vaut
                    // un état « inconnu » qu'une mesure dégradée et un WARN
                    // toutes les 30 s.
                    let statuses = match &adapters {
                        Some(list) if network::is_elevated() => {
                            probe_adapter_statuses(&target, list)
                        }
                        _ => Vec::new(),
                    };

                    (adapters, global, primary, statuses)
                }
            })
            .await;

            // Un JoinError signifie que le thread de travail a paniqué: le tick
            // est abandonné, l'état affiché reste celui du tick précédent.
            let (maybe_adapters, global_connectivity, primary_reachable, statuses) =
                match network_tick {
                    Ok((adapters, global, primary, statuses)) => {
                        (adapters, global, primary, statuses)
                    }
                    Err(e) => {
                        logger::log_file(&format!("ERROR monitor tick join: {e}"));
                        (
                            None,
                            network::ConnectivityResult {
                                reachable: false,
                                latency_ms: None,
                                degraded: false,
                            },
                            None,
                            Vec::new(),
                        )
                    }
                };

            if let Some(adapters) = maybe_adapters {
                // Ne journalise que lorsque la liste change réellement: une
                // ligne « 3 found » toutes les 30 s pour rien noyait le
                // journal et faisait croire à une activité permanente.
                let signature = adapter_signature(&adapters);
                if last_adapter_signature.as_deref() != Some(&signature) {
                    logger::log_file(&format!(
                        "Monitor adapters refreshed: {} found",
                        adapters.len()
                    ));
                    last_adapter_signature = Some(signature);
                }

                // La carte RNDIS du téléphone: son apparition ou sa
                // disparition est journalisée une seule fois.
                let rndis = adapters
                    .iter()
                    .find(|adapter| adb::is_rndis(&adapter.description))
                    .map(|adapter| adapter.name.clone());
                if rndis != last_rndis {
                    if let Some(name) = &rndis {
                        logger::log_file(&format!("USB: carte '{name}' détectée"));
                    }
                    last_rndis = rndis;
                }

                cached_adapters = adapters;
            }

            // ── Partage USB du téléphone (ADB) ──
            // Android ne garde pas la fonction `rndis` après un débranchement:
            // la commande est rejouée ici, tant qu'aucune carte RNDIS n'est
            // active sur le poste.
            //
            // Le travail part dans un thread détaché: une commande ADB lente
            // ne doit pas retarder le cycle de monitoring.
            if usb_tether_auto
                && !adb_busy.load(Ordering::Relaxed)
                && last_usb_check.elapsed() >= Duration::from_secs(USB_TETHER_CHECK_SECS)
            {
                last_usb_check = std::time::Instant::now();
                let adb_path = adb_path.clone();
                let armed_flag = usb_armed.clone();
                let serial_store = usb_serial.clone();
                let busy = adb_busy.clone();
                let warning_store = usb_warning.clone();

                adb_busy.store(true, Ordering::Relaxed);
                tokio::task::spawn_blocking(move || {
                    let outcome = match adb::Adb::open(&adb_path) {
                        Ok(mut client) => client.check(),
                        Err(reason) => adb::Outcome::Failed(reason),
                    };

                    match outcome {
                        adb::Outcome::Armed(serial) => {
                            *serial_store.lock().unwrap_or_else(|e| e.into_inner()) = Some(serial);
                            armed_flag.store(true, Ordering::Relaxed);
                        }
                        adb::Outcome::Failed(reason) => {
                            // Un même avertissement n'est pas rejoué: le
                            // journal se remplissait d'une ligne par tick.
                            let mut last = warning_store.lock().unwrap_or_else(|e| e.into_inner());
                            if last.as_deref() != Some(reason.as_str()) {
                                logger::log_file(&format!("WARN partage USB: {reason}"));
                                *last = Some(reason);
                            }
                        }
                        _ => {}
                    }
                    busy.store(false, Ordering::Relaxed);
                });
            }

            // Connectivité affichée: hystérésis pour ne pas clignoter sur un
            // ping perdu isolé. Le changement est journalisé une seule fois.
            match reachability.observe(global_connectivity.reachable) {
                ReachabilityChange::Online => logger::log_file("Monitor: connectivité rétablie"),
                ReachabilityChange::Offline => logger::log_file("Monitor: perte de connectivité"),
                ReachabilityChange::None => {}
            }

            // Le dernier statut connu est conservé entre deux rafraîchissements.
            if !statuses.is_empty() {
                cached_statuses = statuses;
            }

            // ── Failover: décision sous verrou, application hors verrou ──
            let mut failover_action = String::new();
            let mut metrics_to_set: Vec<(u32, u32)> = Vec::new();
            let mut failed_over_update: Option<bool> = None;

            {
                let state_guard = state.read().await;
                let fo = &state_guard.failover;

                // Le compteur repart de zéro quand le failover vient d'être
                // activé ou désactivé par l'utilisateur.
                if fo.config.enabled != last_failover_enabled {
                    counters.reset();
                    last_failover_enabled = fo.config.enabled;
                }

                // La bascule ne se décide que sur la mesure du primaire.
                // `None` = sonde impossible (failover désactivé, ou primaire
                // sans passerelle): aucune décision n'est prise sur ce tick.
                if primary_reachable.is_none() {
                    counters.reset();
                }

                let action = match primary_reachable {
                    Some(reachable) => decide(fo, &mut counters, reachable),
                    None => FailoverAction::None,
                };

                match action {
                    FailoverAction::None => {}
                    FailoverAction::SwitchToSecondary => {
                        metrics_to_set.push((fo.config.secondary_index, network::METRIC_PRIMARY));
                        metrics_to_set.push((fo.config.primary_index, network::METRIC_SECONDARY));
                        failed_over_update = Some(true);
                        failover_action = format!("🔄 Failover → {}", fo.config.secondary_name);
                    }
                    FailoverAction::RestorePrimary => {
                        metrics_to_set.push((fo.config.primary_index, network::METRIC_PRIMARY));
                        metrics_to_set.push((fo.config.secondary_index, network::METRIC_SECONDARY));
                        failed_over_update = Some(false);
                        failover_action = format!("✅ Restauré → {}", fo.config.primary_name);
                    }
                }
            }

            // Application des métriques SANS maintenir le verrou.
            // L'état n'est mis à jour que si TOUTES les métriques ont été
            // appliquées: sinon `is_failed_over` resterait cohérent avec la
            // table de routage, et la bascule est retentée au tick suivant.
            let metrics_applied = apply_metrics(&metrics_to_set).await;

            // Métriques nominales à rétablir si le failover a été désactivé
            // pendant l'application (elle dure plusieurs secondes, hors verrou).
            let mut revert_nominal: Option<(u32, u32)> = None;
            if let Some(failed_over) = failed_over_update {
                let mut state_guard = state.write().await;
                let fo = &mut state_guard.failover;

                // Vérifié SOUS verrou d'écriture, en même temps que la publication:
                // `disable_auto_failover` a pu passer `enabled` à false sans voir
                // la bascule en cours. On ne publie alors rien et on remet les
                // métriques nominales, sinon le secondaire resterait prioritaire
                // sans que plus rien ne le restaure.
                if !fo.config.enabled {
                    revert_nominal = Some((fo.config.primary_index, fo.config.secondary_index));
                } else if metrics_applied {
                    if fo.is_failed_over != failed_over {
                        fo.last_switch = now_label();

                        if failed_over {
                            fo.failover_count += 1;
                            logger::log_file(&format!(
                                "FAILOVER: bascule sur '{}' ({})",
                                fo.config.secondary_name, fo.config.secondary_index
                            ));
                        } else {
                            logger::log_file(&format!(
                                "FAILOVER RECOVERY: retour sur '{}' ({})",
                                fo.config.primary_name, fo.config.primary_index
                            ));
                        }
                    }

                    fo.is_failed_over = failed_over;
                } else {
                    logger::log_file(&format!(
                        "ERROR {} ignoré: application des métriques échouée",
                        if failed_over {
                            "failover"
                        } else {
                            "retour primaire"
                        }
                    ));

                    // `decide()` a remis les compteurs à zéro en décidant
                    // l'action. On les réarme pour que la bascule soit
                    // retentie au tick suivant plutôt que dans deux ticks.
                    if failed_over {
                        counters.failures = FAILOVER_THRESHOLD;
                        counters.successes = 0;
                    } else {
                        counters.successes = RECOVERY_THRESHOLD;
                        counters.failures = 0;
                    }
                }
            }

            if let Some((primary, secondary)) = revert_nominal {
                logger::log_file(
                    "FAILOVER: désactivé pendant la bascule, métriques nominales rétablies",
                );
                if !apply_metrics(&[
                    (primary, network::METRIC_PRIMARY),
                    (secondary, network::METRIC_SECONDARY),
                ])
                .await
                {
                    logger::log_file("ERROR rétablissement des métriques nominales incomplet");
                }
            }

            let now = now_label();

            // ── Publication de l'état ──
            let event_payload = {
                let mut state_guard = state.write().await;
                state_guard.internet_reachable = reachability.is_online();
                state_guard.internet_known = reachability.is_known();
                state_guard.overall_latency_ms = if reachability.is_online() {
                    global_connectivity.latency_ms
                } else {
                    None
                };
                state_guard.adapters = cached_statuses.clone();
                state_guard.adapter_names =
                    cached_adapters.iter().map(|a| a.name.clone()).collect();
                state_guard.last_check = now.clone();

                let mut payload = MonitorState {
                    internet_reachable: state_guard.internet_reachable,
                    internet_known: state_guard.internet_known,
                    overall_latency_ms: state_guard.overall_latency_ms,
                    adapters: state_guard.adapters.clone(),
                    adapter_names: state_guard.adapter_names.clone(),
                    last_check: state_guard.last_check.clone(),
                    failover: state_guard.failover.clone(),
                };
                if !failover_action.is_empty() {
                    payload.last_check = format!("{now} | {failover_action}");
                }
                payload
            };

            if let Err(e) = app_handle.emit("monitoring-update", &event_payload) {
                logger::log_file(&format!("ERROR emit monitoring-update: {e}"));
            }

            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    });
}

/// Empreinte de la liste d'adaptateurs pour ne journaliser que les
/// changements réels (nom, index, statut).
fn adapter_signature(adapters: &[network::NetworkAdapter]) -> String {
    adapters
        .iter()
        .map(|a| format!("{}|{}|{}", a.interface_index, a.name, a.status))
        .collect::<Vec<_>>()
        .join(";")
}

/// Interfaces réellement sondables: connectées, avec une IP et une passerelle
/// IPv4 valides.
///
/// Un adaptateur virtuel (Hyper-V, VMware, WSL) est « Up » sans passerelle par
/// défaut. Le sonder produisait un WARN de repli `ping -S` toutes les 30 s et
/// un « Déconnecté » mensonger: il n'a pas de route à tester, son état reste
/// inconnu.
fn probe_candidates(
    adapters: &[network::NetworkAdapter],
    details: &[network::AdapterDetails],
) -> Vec<network::RouteTarget> {
    adapters
        .iter()
        .filter(|a| a.is_connected)
        .filter_map(|a| {
            details
                .iter()
                .find(|d| d.interface_index == a.interface_index)
        })
        .filter(|d| network::validate_gateway(d.default_gateway.trim()))
        .filter(|d| !d.ip_address.trim().is_empty() && d.ip_address != "N/A")
        .map(|d| network::RouteTarget {
            interface_index: d.interface_index,
            source_ip: Some(d.ip_address.clone()),
            gateway: Some(d.default_gateway.clone()),
        })
        .collect()
}

/// Traduit une sonde en statut affichable.
///
/// Absente ou dégradée => `status_known: false`: l'UI affiche « État inconnu »
/// au lieu de prétendre que l'adaptateur est déconnecté.
fn status_for(
    adapter: &network::NetworkAdapter,
    probe: Option<&network::ConnectivityResult>,
) -> AdapterStatus {
    let usable = probe.filter(|r| !r.degraded);

    AdapterStatus {
        name: adapter.name.clone(),
        reachable: usable.is_some_and(|r| r.reachable),
        latency_ms: usable.and_then(|r| r.latency_ms),
        status_known: usable.is_some(),
    }
}

//     8.8.8.8 ─┬─ route épinglée (ifA) ─> ping A ─> cible
//              └─ route épinglée (ifB) ─> ping B ─> cible
//
/// Teste chaque adaptateur connecté via une route hôte épinglée sur lui-même.
///
/// Un ping global mesure surtout la route par défaut: il ne dit rien de la
/// santé d'un adaptateur donné. Les sondes sont séquentielles, une seule route
/// `/32` servant à la fois.
fn probe_adapter_statuses(
    target: &str,
    adapters: &[network::NetworkAdapter],
) -> Vec<AdapterStatus> {
    let connected: Vec<&network::NetworkAdapter> =
        adapters.iter().filter(|a| a.is_connected).collect();

    if connected.is_empty() {
        return Vec::new();
    }

    // Une seule requête de détails pour tous les adaptateurs, puis un ping chacun.
    let details = match network::get_all_adapter_details() {
        Ok(d) => d,
        Err(e) => {
            logger::log_file(&format!("ERROR get_all_adapter_details (monitor): {e:#}"));
            return Vec::new();
        }
    };

    let routes = probe_candidates(adapters, &details);
    let probes = network::probe_routes(target, &routes);

    connected
        .iter()
        .map(|adapter| {
            let probe = probes
                .iter()
                .find(|(index, _)| *index == adapter.interface_index)
                .map(|(_, result)| result);

            status_for(adapter, probe)
        })
        .collect()
}

/// Applique une liste de métriques. Retourne `true` seulement si toutes ont
/// réussi: un échec partiel laisse la table de routage incohérente, il est
/// donc remonté tel quel au lieu d'être masqué.
async fn apply_metrics(metrics: &[(u32, u32)]) -> bool {
    if metrics.is_empty() {
        return true;
    }

    let targets = metrics.to_vec();
    let result = tokio::task::spawn_blocking(move || {
        let mut all_ok = true;

        for (index, metric) in targets {
            if let Err(e) = network::set_routing_metric(index, metric) {
                logger::log_file(&format!(
                    "ERROR set_routing_metric({index}, {metric}): {e:#}"
                ));
                all_ok = false;
            }
        }

        all_ok
    })
    .await;

    match result {
        Ok(ok) => ok,
        Err(e) => {
            logger::log_file(&format!("ERROR apply_metrics join: {e}"));
            false
        }
    }
}

fn now_label() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(enabled: bool, is_failed_over: bool) -> FailoverState {
        FailoverState {
            config: FailoverConfig {
                enabled,
                ..Default::default()
            },
            is_failed_over,
            ..Default::default()
        }
    }

    #[test]
    fn pas_de_bascule_apres_un_seul_echec() {
        let mut c = FailoverCounters::default();
        assert_eq!(
            decide(&state(true, false), &mut c, false),
            FailoverAction::None
        );
        assert_eq!(c.failures, 1);
    }

    #[test]
    fn bascule_apres_deux_echecs_consecutifs() {
        let mut c = FailoverCounters::default();
        let s = state(true, false);

        assert_eq!(decide(&s, &mut c, false), FailoverAction::None);
        assert_eq!(decide(&s, &mut c, false), FailoverAction::SwitchToSecondary);
    }

    #[test]
    fn un_succes_isole_ne_restaure_pas() {
        let mut c = FailoverCounters::default();
        let s = state(true, true);

        assert_eq!(decide(&s, &mut c, true), FailoverAction::None);
        assert_eq!(c.successes, 1);
        assert_eq!(decide(&s, &mut c, false), FailoverAction::None);
        assert_eq!(decide(&s, &mut c, true), FailoverAction::None);
        assert_eq!(decide(&s, &mut c, true), FailoverAction::None);
        assert_eq!(decide(&s, &mut c, true), FailoverAction::RestorePrimary);
    }

    #[test]
    fn restauration_apres_trois_succes_consecutifs() {
        let mut c = FailoverCounters::default();
        let s = state(true, true);

        assert_eq!(decide(&s, &mut c, true), FailoverAction::None);
        assert_eq!(decide(&s, &mut c, true), FailoverAction::None);
        assert_eq!(decide(&s, &mut c, true), FailoverAction::RestorePrimary);
    }

    #[test]
    fn pas_de_flapping_alternance_echec_succes() {
        let mut c = FailoverCounters::default();
        let s = state(true, false);

        for _ in 0..20 {
            assert_eq!(decide(&s, &mut c, false), FailoverAction::None);
            assert_eq!(decide(&s, &mut c, true), FailoverAction::None);
        }
    }

    #[test]
    fn pas_de_flapping_pendant_un_failover_actif() {
        let mut c = FailoverCounters::default();
        let s = state(true, true);

        // Pendant un failover actif, le primaire reste coupé: une mesure
        // réussie isolée ne doit pas le restaurer.
        for _ in 0..5 {
            assert_eq!(decide(&s, &mut c, false), FailoverAction::None);
            assert_eq!(decide(&s, &mut c, true), FailoverAction::None);
        }
    }

    #[test]
    fn compteurs_remis_a_zero_au_changement_d_etat() {
        let mut c = FailoverCounters::default();
        let s = state(true, false);

        decide(&s, &mut c, false);
        assert_eq!(decide(&s, &mut c, false), FailoverAction::SwitchToSecondary);
        assert_eq!(c, FailoverCounters::default());

        let failed_over = state(true, true);
        decide(&failed_over, &mut c, true);
        decide(&failed_over, &mut c, true);
        assert_eq!(
            decide(&failed_over, &mut c, true),
            FailoverAction::RestorePrimary
        );
        assert_eq!(c, FailoverCounters::default());
    }

    #[test]
    fn compteurs_reinitialises_quand_failover_desactive() {
        let mut c = FailoverCounters::default();
        let s = state(true, false);

        decide(&s, &mut c, false);
        assert_eq!(c.failures, 1);

        let disabled = state(false, false);
        assert_eq!(decide(&disabled, &mut c, false), FailoverAction::None);
        assert_eq!(c, FailoverCounters::default());
    }

    #[test]
    fn seuils_sont_ceux_attendus() {
        assert_eq!(FAILOVER_THRESHOLD, 2);
        assert_eq!(RECOVERY_THRESHOLD, 3);
    }

    #[test]
    fn adapter_status_deserialise_sans_status_known() {
        // Compatibilité: un état sérialisé par une version antérieure (ou un
        // frontend qui ignore le champ) doit rester lisible, avec l'état
        // « connu » par défaut.
        let legacy = r#"{"name": "Wi-Fi", "reachable": true, "latency_ms": 12}"#;
        let parsed: AdapterStatus = serde_json::from_str(legacy).unwrap();
        assert!(parsed.status_known);

        let unknown =
            r#"{"name": "Wi-Fi", "reachable": false, "latency_ms": null, "status_known": false}"#;
        let parsed: AdapterStatus = serde_json::from_str(unknown).unwrap();
        assert!(!parsed.status_known);
    }

    #[test]
    fn sauvegarde_atomique_sans_residu() {
        let dir = std::env::temp_dir().join(format!("dualink-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");

        // Deux écritures: la seconde remplace la première, sans résidu .tmp.
        write_atomic(&path, "{\"interval_secs\":5}").unwrap();
        write_atomic(&path, "{\"interval_secs\":30}").unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "{\"interval_secs\":30}"
        );
        assert!(!dir.join("settings.json.tmp").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_validees_au_chargement() {
        let too_fast = Settings {
            interval_secs: 0,
            ping_target: String::new(),
            adapter_refresh_secs: 1,
            ..Settings::default()
        }
        .validated();
        assert_eq!(too_fast.interval_secs, MIN_INTERVAL_SECS);
        assert_eq!(too_fast.adapter_refresh_secs, MIN_REFRESH_SECS);
        assert_eq!(too_fast.ping_target, network::DEFAULT_PING_TARGET);

        let too_slow = Settings {
            interval_secs: 10_000,
            ping_target: "  1.1.1.1  ".to_string(),
            adapter_refresh_secs: 10_000,
            ..Settings::default()
        }
        .validated();
        assert_eq!(too_slow.interval_secs, MAX_INTERVAL_SECS);
        assert_eq!(too_slow.adapter_refresh_secs, MAX_REFRESH_SECS);
        assert_eq!(too_slow.ping_target, "1.1.1.1");

        let injectable = Settings {
            interval_secs: 5,
            ping_target: "8.8.8.8 & calc.exe".to_string(),
            adapter_refresh_secs: 30,
            ..Settings::default()
        }
        .validated();
        assert_eq!(injectable.ping_target, network::DEFAULT_PING_TARGET);
    }

    #[test]
    fn reglages_adb_par_defaut_et_bornes() {
        let defaults = Settings::default();
        assert!(defaults.usb_tether_auto);
        assert_eq!(defaults.adb_path, "");

        // Un fichier écrit avant l'option ADB doit rester lisible.
        let legacy: Settings = serde_json::from_str(
            r#"{"interval_secs":5,"ping_target":"1.1.1.1","adapter_refresh_secs":30}"#,
        )
        .unwrap();
        assert!(legacy.usb_tether_auto);

        let path = Settings {
            adb_path: "  C:\\Program Files\\adb.exe  ".to_string(),
            ..Settings::default()
        }
        .validated();
        assert_eq!(path.adb_path, "C:\\Program Files\\adb.exe");
    }

    fn adapter(name: &str, description: &str, status: &str) -> network::NetworkAdapter {
        network::NetworkAdapter {
            name: name.to_string(),
            interface_index: 0,
            status: status.to_string(),
            mac_address: "N/A".to_string(),
            speed: "N/A".to_string(),
            description: description.to_string(),
            is_connected: status == "Up",
        }
    }

    #[test]
    fn un_ping_perdu_ne_fait_pas_clignoter_l_etat() {
        let mut r = Reachability::default();

        assert_eq!(r.observe(true), ReachabilityChange::Online);
        assert!(r.is_online());

        // Un échec isolé (Wi-Fi, changement de route pendant une bascule)
        // laisse l'en-tête « Connecté ».
        assert_eq!(r.observe(false), ReachabilityChange::None);
        assert!(r.is_online());

        assert_eq!(r.observe(false), ReachabilityChange::Offline);
        assert!(!r.is_online());

        assert_eq!(r.observe(true), ReachabilityChange::Online);
        assert!(r.is_online());
    }

    #[test]
    fn etat_inconnu_avant_la_premiere_mesure() {
        let mut r = Reachability::default();
        assert!(!r.is_known());

        assert_eq!(r.observe(false), ReachabilityChange::None);
        assert!(!r.is_known());
        assert!(!r.is_online());

        assert_eq!(r.observe(false), ReachabilityChange::Offline);
        assert!(r.is_known());
        assert!(!r.is_online());
    }

    #[test]
    fn un_succes_efface_les_echecs_accumules() {
        let mut r = Reachability::default();
        assert_eq!(r.observe(true), ReachabilityChange::Online);

        assert_eq!(r.observe(false), ReachabilityChange::None);
        assert_eq!(r.observe(true), ReachabilityChange::None);
        assert_eq!(r.observe(false), ReachabilityChange::None);
        assert!(r.is_online());

        assert_eq!(r.observe(false), ReachabilityChange::Offline);
    }

    #[test]
    fn sonde_utilise_la_cible_resolue() {
        let mut config = FailoverConfig {
            enabled: true,
            probe_target: "1.1.1.1".to_string(),
            ..Default::default()
        };

        // La cible du réglage peut être un nom d'hôte: la sonde doit utiliser
        // l'IPv4 résolue, seule adresse acceptable pour une route /32.
        assert_eq!(probe_host(&config, "dns.google"), "1.1.1.1");

        // Configuration héritée (pas de champ): repli sur la cible du tick.
        config.probe_target.clear();
        assert_eq!(probe_host(&config, "1.1.1.1"), "1.1.1.1");
    }

    #[test]
    fn signature_adaptateurs_stable_et_sensible_au_statut() {
        let a = vec![adapter("Wi-Fi", "Realtek", "Up")];
        let b = vec![adapter("Wi-Fi", "Realtek", "Up")];
        assert_eq!(adapter_signature(&a), adapter_signature(&b));

        let c = vec![adapter("Wi-Fi", "Realtek", "Disabled")];
        assert_ne!(adapter_signature(&a), adapter_signature(&c));
    }

    fn details(name: &str, index: u32, ip: &str, gateway: &str) -> network::AdapterDetails {
        network::AdapterDetails {
            name: name.to_string(),
            interface_index: index,
            status: "Up".to_string(),
            description: "desc".to_string(),
            mac_address: "N/A".to_string(),
            speed: "N/A".to_string(),
            ip_address: ip.to_string(),
            subnet_mask: "255.255.255.0".to_string(),
            default_gateway: gateway.to_string(),
            dns_servers: "N/A".to_string(),
            dhcp_enabled: true,
            routing_metric: None,
        }
    }

    fn up_adapter(name: &str, index: u32) -> network::NetworkAdapter {
        network::NetworkAdapter {
            name: name.to_string(),
            interface_index: index,
            status: "Up".to_string(),
            mac_address: "N/A".to_string(),
            speed: "N/A".to_string(),
            description: "desc".to_string(),
            is_connected: true,
        }
    }

    #[test]
    fn ne_sonde_pas_un_adaptateur_sans_passerelle() {
        let adapters = vec![up_adapter("Wi-Fi", 1), up_adapter("vEthernet", 2)];
        let details = vec![
            details("Wi-Fi", 1, "192.168.1.10", "192.168.1.1"),
            details("vEthernet", 2, "172.28.32.1", ""),
        ];

        let routes = probe_candidates(&adapters, &details);

        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].interface_index, 1);
    }

    #[test]
    fn statut_inconnu_quand_la_sonde_manque() {
        let a = up_adapter("vEthernet", 2);
        let s = status_for(&a, None);

        assert!(!s.reachable);
        assert!(!s.status_known);
        assert_eq!(s.latency_ms, None);
    }

    #[test]
    fn statut_degrade_reste_inconnu() {
        let a = up_adapter("Wi-Fi", 1);
        let probe = network::ConnectivityResult {
            reachable: true,
            latency_ms: Some(12),
            degraded: true,
        };

        let s = status_for(&a, Some(&probe));

        assert!(!s.reachable);
        assert!(!s.status_known);
        assert_eq!(s.latency_ms, None);
    }

    #[test]
    fn statut_connu_quand_la_sonde_est_fiable() {
        let a = up_adapter("Wi-Fi", 1);
        let probe = network::ConnectivityResult {
            reachable: true,
            latency_ms: Some(12),
            degraded: false,
        };

        let s = status_for(&a, Some(&probe));

        assert!(s.reachable);
        assert!(s.status_known);
        assert_eq!(s.latency_ms, Some(12));
    }
}
