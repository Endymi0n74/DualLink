use crate::logger;
use crate::network;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::RwLock;

/// Nombre d'échecs consécutifs requis pour basculer sur le secondaire.
pub const FAILOVER_THRESHOLD: u32 = 2;
/// Nombre de succès consécutifs requis pour restaurer le primaire.
pub const RECOVERY_THRESHOLD: u32 = 3;

/// Bornes de validation des réglages.
const MIN_INTERVAL_SECS: u64 = 1;
const MAX_INTERVAL_SECS: u64 = 300;
const MIN_REFRESH_SECS: u64 = 5;
const MAX_REFRESH_SECS: u64 = 600;

// ─── Settings ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub interval_secs: u64,
    pub ping_target: String,
    pub adapter_refresh_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            interval_secs: 5,
            ping_target: network::DEFAULT_PING_TARGET.to_string(),
            adapter_refresh_secs: 30,
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
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }
}

fn dirs() -> Option<std::path::PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(|base| {
        let mut p = std::path::PathBuf::from(base);
        p.push("DualLink");
        p
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterStatus {
    pub name: String,
    pub reachable: bool,
    pub latency_ms: Option<u64>,
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
    pub overall_latency_ms: Option<u64>,
    pub adapters: Vec<AdapterStatus>,
    pub last_check: String,
    pub failover: FailoverState,
}

impl Default for MonitorState {
    fn default() -> Self {
        Self {
            internet_reachable: false,
            overall_latency_ms: None,
            adapters: Vec::new(),
            last_check: "Never".to_string(),
            failover: FailoverState::default(),
        }
    }
}

// ─── Sonde du primaire ────────────────────────────────────────────

/// Teste la joignabilité du PRIMAIRE via une route hôte épinglée.
///
/// Retourne `None` quand la sonde n'a pas lieu d'être: failover désactivé, ou
/// primaire sans passerelle connue (impossible d'épingler la route). Dans ce
/// cas l'appelant ne prend aucune décision, plutôt que de décider sur une
/// mesure qui ne teste pas le primaire.
fn probe_primary(target: &str, config: &FailoverConfig) -> Option<bool> {
    if !config.enabled {
        return None;
    }

    let gateway = config.primary_gateway.as_deref()?;
    if gateway.trim().is_empty() || gateway == "N/A" {
        logger::log_file("WARN sonde primaire: passerelle inconnue, décision suspendue");
        return None;
    }

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
        let mut last_failover_enabled = false;

        loop {
            tick_count += 1;

            let Settings {
                interval_secs,
                ping_target,
                adapter_refresh_secs,
            } = settings.read().await.clone().validated();

            let refresh_interval = (adapter_refresh_secs / interval_secs).max(1);

            // ── Failover: état lu AVANT les mesures ──
            // La sonde du primaire a besoin de sa passerelle pour épingler la
            // route: on lit donc la configuration avant de lancer les I/O.
            let failover_cfg = state.read().await.failover.config.clone();

            // ── Mesures réseau (bloquantes: PowerShell + ping.exe) ──
            // Exécutées dans un thread dédié pour ne pas bloquer le runtime
            // async de Tauri (commandes et événements partagent ce runtime).
            let adapters_fresh = tick_count % refresh_interval == 1 || cached_adapters.is_empty();
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
                    let statuses = match &adapters {
                        Some(list) => probe_adapter_statuses(&target, list),
                        None => Vec::new(),
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
                            },
                            None,
                            Vec::new(),
                        )
                    }
                };

            if let Some(adapters) = maybe_adapters {
                logger::log_file(&format!(
                    "Monitor adapters refreshed: {} found",
                    adapters.len()
                ));
                cached_adapters = adapters;
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

            if let Some(failed_over) = failed_over_update {
                if metrics_applied {
                    let mut state_guard = state.write().await;
                    let fo = &mut state_guard.failover;

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

            let now = now_label();

            // ── Publication de l'état ──
            let event_payload = {
                let mut state_guard = state.write().await;
                state_guard.internet_reachable = global_connectivity.reachable;
                state_guard.overall_latency_ms = global_connectivity.latency_ms;
                state_guard.adapters = cached_statuses.clone();
                state_guard.last_check = now.clone();

                let mut payload = MonitorState {
                    internet_reachable: state_guard.internet_reachable,
                    overall_latency_ms: state_guard.overall_latency_ms,
                    adapters: state_guard.adapters.clone(),
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

    let routes: Vec<network::RouteTarget> = connected
        .iter()
        .filter_map(|adapter| {
            details
                .iter()
                .find(|d| d.interface_index == adapter.interface_index)
                .map(|d| network::RouteTarget {
                    interface_index: d.interface_index,
                    source_ip: Some(d.ip_address.clone()),
                    gateway: Some(d.default_gateway.clone()),
                })
        })
        .collect();

    let probes = network::probe_routes(target, &routes);

    connected
        .iter()
        .map(|adapter| {
            let probe = probes
                .iter()
                .find(|(index, _)| *index == adapter.interface_index);

            AdapterStatus {
                name: adapter.name.clone(),
                reachable: probe.is_some_and(|(_, r)| r.reachable),
                latency_ms: probe.and_then(|(_, r)| r.latency_ms),
            }
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
    fn settings_validees_au_chargement() {
        let too_fast = Settings {
            interval_secs: 0,
            ping_target: String::new(),
            adapter_refresh_secs: 1,
        }
        .validated();
        assert_eq!(too_fast.interval_secs, MIN_INTERVAL_SECS);
        assert_eq!(too_fast.adapter_refresh_secs, MIN_REFRESH_SECS);
        assert_eq!(too_fast.ping_target, network::DEFAULT_PING_TARGET);

        let too_slow = Settings {
            interval_secs: 10_000,
            ping_target: "  1.1.1.1  ".to_string(),
            adapter_refresh_secs: 10_000,
        }
        .validated();
        assert_eq!(too_slow.interval_secs, MAX_INTERVAL_SECS);
        assert_eq!(too_slow.adapter_refresh_secs, MAX_REFRESH_SECS);
        assert_eq!(too_slow.ping_target, "1.1.1.1");

        let injectable = Settings {
            interval_secs: 5,
            ping_target: "8.8.8.8 & calc.exe".to_string(),
            adapter_refresh_secs: 30,
        }
        .validated();
        assert_eq!(injectable.ping_target, network::DEFAULT_PING_TARGET);
    }
}
