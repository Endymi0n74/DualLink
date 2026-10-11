mod logger;
mod monitor;
mod network;

use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Emitter, Manager,
};
use tokio::sync::RwLock;

use monitor::{FailoverConfig, FailoverState, MonitorState, Settings};
use network::{AdapterDetails, ConnectivityResult, NetworkAdapter};

struct AppState {
    monitor_state: Arc<RwLock<MonitorState>>,
    settings: Arc<RwLock<Settings>>,
}

/// Exécute un appel bloquant (PowerShell, ping.exe) hors du runtime async.
///
/// Les commandes Tauri tournent sur le runtime Tokio: y appeler
/// `Command::output()` bloquerait un worker du pool, donc toutes les autres
/// commandes et l'émission des événements de monitoring. La fermeture doit
/// renvoyer `Result<T, String>`: les erreurs sont formatées sur place et
/// simplement remontées à l'appelant.
async fn blocking<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(f).await {
        Ok(result) => result,
        Err(join_err) => Err(format!("Blocking task failed: {join_err}")),
    }
}

/// Journalise une commande réseau qui renvoie `Result<(), String>`.
fn log_unit_result(cmd: &str, result: &Result<(), String>) {
    if let Err(e) = result {
        logger::log_file(&format!("CMD {cmd} ERROR: {e}"));
    }
}

// ─── Tauri Commands ───────────────────────────────────────────────

#[tauri::command]
async fn list_adapters() -> Result<Vec<NetworkAdapter>, String> {
    logger::log_file("CMD list_adapters");

    let result = blocking(|| network::list_adapters().map_err(|e| format!("{e:#}"))).await;

    match &result {
        Ok(adapters) => logger::log_file(&format!("CMD list_adapters: {} found", adapters.len())),
        Err(e) => logger::log_file(&format!("CMD list_adapters ERROR: {e}")),
    }

    result
}

#[tauri::command]
async fn enable_adapter(name: String) -> Result<(), String> {
    logger::log_file(&format!("CMD enable_adapter: {name}"));
    let label = format!("enable_adapter '{name}'");

    let result =
        blocking(move || network::enable_adapter(&name).map_err(|e| format!("{e:#}"))).await;

    log_unit_result(&label, &result);
    result
}

#[tauri::command]
async fn disable_adapter(name: String) -> Result<(), String> {
    logger::log_file(&format!("CMD disable_adapter: {name}"));
    let label = format!("disable_adapter '{name}'");

    let result =
        blocking(move || network::disable_adapter(&name).map_err(|e| format!("{e:#}"))).await;

    log_unit_result(&label, &result);
    result
}

#[tauri::command]
async fn test_connectivity() -> Result<ConnectivityResult, String> {
    logger::log_file("CMD test_connectivity");

    let result = blocking(|| network::test_connectivity().map_err(|e| format!("{e:#}"))).await;

    if let Err(e) = &result {
        logger::log_file(&format!("CMD test_connectivity ERROR: {e}"));
    }

    result
}

#[tauri::command]
async fn test_adapter_connectivity(name: String) -> Result<ConnectivityResult, String> {
    logger::log_file(&format!("CMD test_adapter_connectivity: {name}"));
    let label = format!("test_adapter_connectivity '{name}'");

    let result =
        blocking(move || network::test_adapter_connectivity(&name).map_err(|e| format!("{e:#}")))
            .await;

    if let Err(e) = &result {
        logger::log_file(&format!("CMD {label} ERROR: {e}"));
    }

    result
}

#[tauri::command]
async fn set_routing_metric(interface_index: u32, metric: u32) -> Result<(), String> {
    logger::log_file(&format!(
        "CMD set_routing_metric: {interface_index} -> {metric}"
    ));

    let result = blocking(move || {
        network::set_routing_metric(interface_index, metric).map_err(|e| format!("{e:#}"))
    })
    .await;

    log_unit_result(
        &format!("set_routing_metric({interface_index}, {metric})"),
        &result,
    );
    result
}

#[tauri::command]
async fn configure_load_balancing(primary_index: u32, secondary_index: u32) -> Result<(), String> {
    logger::log_file(&format!(
        "CMD configure_load_balancing: {primary_index} / {secondary_index}"
    ));

    // Ce n'est pas du load balancing: une destination ne suit qu'une seule
    // interface, la plus basse métrique. C'est une répartition de priorité.
    let result = blocking(move || {
        network::configure_load_balancing(primary_index, secondary_index)
            .map_err(|e| format!("{e:#}"))
    })
    .await;

    log_unit_result(
        &format!("configure_load_balancing({primary_index}, {secondary_index})"),
        &result,
    );
    result
}

#[tauri::command]
async fn configure_failover(primary_index: u32, secondary_index: u32) -> Result<(), String> {
    logger::log_file(&format!(
        "CMD configure_failover: {primary_index} / {secondary_index}"
    ));

    let result = blocking(move || {
        network::configure_failover(primary_index, secondary_index).map_err(|e| format!("{e:#}"))
    })
    .await;

    log_unit_result(
        &format!("configure_failover({primary_index}, {secondary_index})"),
        &result,
    );
    result
}

#[tauri::command]
async fn get_monitor_state(state: tauri::State<'_, AppState>) -> Result<MonitorState, String> {
    let monitor = state.monitor_state.read().await;
    Ok(monitor.clone())
}

#[tauri::command]
async fn enable_auto_failover(
    state: tauri::State<'_, AppState>,
    primary_index: u32,
    secondary_index: u32,
    primary_name: String,
    secondary_name: String,
) -> Result<FailoverState, String> {
    logger::log_file(&format!(
        "CMD enable_auto_failover: {primary_name}({primary_index}) -> {secondary_name}({secondary_index})"
    ));

    // Résolution des identifiants de route du primaire AVANT toute modification:
    // sans IP ni passerelle, la route hôte de test ne peut pas être créée et le
    // failover testerait la route par défaut, donc le secondaire. Mieux vaut
    // refuser l'activation que d'activer un failover qui ne fonctionne pas.
    let primary_details = blocking({
        let name = primary_name.clone();
        move || network::get_adapter_details(&name).map_err(|e| format!("{e:#}"))
    })
    .await?;

    if primary_details.default_gateway.trim().is_empty() || primary_details.default_gateway == "N/A"
    {
        let msg = format!(
            "Passerelle introuvable sur '{primary_name}' (if {primary_index}): \
             le failover automatique ne peut pas être activé"
        );
        logger::log_file(&format!("ERROR enable_auto_failover: {msg}"));
        return Err(msg);
    }

    if primary_details.ip_address.trim().is_empty() || primary_details.ip_address == "N/A" {
        let msg = format!(
            "Adresse IP introuvable sur '{primary_name}' (if {primary_index}): \
             le failover automatique ne peut pas être activé"
        );
        logger::log_file(&format!("ERROR enable_auto_failover: {msg}"));
        return Err(msg);
    }

    let gateway = primary_details.default_gateway.clone();
    let primary_ip = primary_details.ip_address.clone();

    // Métriques initiales: primaire=10, secondaire=100. Un échec ici est
    // remonté au frontend plutôt que d'ignoré: le failover resterait inactif
    // côté table de routage alors que l'UI afficherait « standby ».
    blocking(move || {
        network::set_routing_metric(primary_index, network::METRIC_PRIMARY)
            .map_err(|e| format!("{e:#}"))?;
        network::set_routing_metric(secondary_index, network::METRIC_SECONDARY)
            .map_err(|e| format!("{e:#}"))
    })
    .await?;

    let mut monitor = state.monitor_state.write().await;
    monitor.failover = FailoverState {
        config: FailoverConfig {
            enabled: true,
            primary_index,
            secondary_index,
            primary_name,
            secondary_name,
            primary_gateway: Some(gateway),
            primary_ip: Some(primary_ip),
        },
        is_failed_over: false,
        failover_count: 0,
        last_switch: "Never".to_string(),
    };
    Ok(monitor.failover.clone())
}

#[tauri::command]
async fn disable_auto_failover(state: tauri::State<'_, AppState>) -> Result<FailoverState, String> {
    logger::log_file("CMD disable_auto_failover");

    // Lecture de l'état sous verrou, puis libération avant toute I/O réseau.
    let (was_failed_over, primary_idx, secondary_idx) = {
        let monitor = state.monitor_state.read().await;
        (
            monitor.failover.is_failed_over,
            monitor.failover.config.primary_index,
            monitor.failover.config.secondary_index,
        )
    };

    if was_failed_over {
        blocking(move || {
            network::set_routing_metric(primary_idx, network::METRIC_PRIMARY)
                .map_err(|e| format!("{e:#}"))?;
            network::set_routing_metric(secondary_idx, network::METRIC_SECONDARY)
                .map_err(|e| format!("{e:#}"))
        })
        .await?;

        logger::log_file("CMD disable_auto_failover: restored primary metrics");
    }

    let mut monitor = state.monitor_state.write().await;
    monitor.failover.config.enabled = false;
    monitor.failover.is_failed_over = false;
    Ok(monitor.failover.clone())
}

#[tauri::command]
async fn get_failover_state(state: tauri::State<'_, AppState>) -> Result<FailoverState, String> {
    let monitor = state.monitor_state.read().await;
    Ok(monitor.failover.clone())
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    let s = state.settings.read().await;
    Ok(s.clone())
}

#[tauri::command]
fn get_logs() -> Result<String, String> {
    Ok(logger::read_today_log())
}

/// Version de l'application, lue depuis `Cargo.toml`.
///
/// Le frontend affiche cette valeur: la écrire en dur dans le JS obligerait
/// à toucher deux fichiers à chaque release, et l'affichage finirait par
/// mentir. `CARGO_PKG_VERSION` suit le bump de version du crate.
#[tauri::command]
fn get_app_version() -> Result<String, String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[tauri::command]
fn get_log_files() -> Result<Vec<String>, String> {
    Ok(logger::list_log_files())
}

#[tauri::command]
fn get_log_by_date(date: String) -> Result<String, String> {
    Ok(logger::read_log_file(&date))
}

#[tauri::command]
async fn get_adapter_details(name: String) -> Result<AdapterDetails, String> {
    logger::log_file(&format!("CMD get_adapter_details: {name}"));
    let label = format!("get_adapter_details '{name}'");

    let result =
        blocking(move || network::get_adapter_details(&name).map_err(|e| format!("{e:#}"))).await;

    if let Err(e) = &result {
        logger::log_file(&format!("CMD {label} ERROR: {e}"));
    }

    result
}

#[tauri::command]
async fn get_all_adapter_details() -> Result<Vec<AdapterDetails>, String> {
    logger::log_file("CMD get_all_adapter_details");

    let result =
        blocking(|| network::get_all_adapter_details().map_err(|e| format!("{e:#}"))).await;

    if let Err(e) = &result {
        logger::log_file(&format!("CMD get_all_adapter_details ERROR: {e}"));
    }

    result
}

#[tauri::command]
async fn save_settings(
    state: tauri::State<'_, AppState>,
    interval_secs: u64,
    ping_target: String,
    adapter_refresh_secs: u64,
) -> Result<Settings, String> {
    logger::log_file(&format!(
        "CMD save_settings: interval={interval_secs}s target={ping_target} refresh={adapter_refresh_secs}s"
    ));

    // La validation est appliquée ici comme au chargement: `Settings::load`
    // relit ce fichier au prochain démarrage, un fichier non borné aurait donc
    // un effet différent selon le moment où il est relu.
    let requested = Settings {
        interval_secs,
        ping_target: ping_target.clone(),
        adapter_refresh_secs,
    };
    let new_settings = requested.clone().validated();

    if new_settings.interval_secs != requested.interval_secs
        || new_settings.adapter_refresh_secs != requested.adapter_refresh_secs
        || new_settings.ping_target != requested.ping_target
    {
        logger::log_file(&format!(
            "WARN save_settings: valeurs hors bornes corrigées -> {new_settings:?}"
        ));
    }

    new_settings.save()?;

    let mut s = state.settings.write().await;
    *s = new_settings.clone();
    Ok(new_settings)
}

// ─── App Entry ────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::init();
    logger::log_file("DualLink starting...");
    // Sans jeton élevé, toutes les cmdlets Net* échoueront (bug #29):
    // trace l'état dès le démarrage pour diagnostiquer en un coup d'œil.
    logger::log_file(&format!("Elevated: {}", network::is_elevated()));

    let monitor_state = Arc::new(RwLock::new(MonitorState::default()));
    let settings = Arc::new(RwLock::new(Settings::load()));

    tauri::Builder::default()
        // Doit être enregistré en premier: le plugin prend le verrou d'instance
        // unique au démarrage. Un 2e lancement transmet ses arguments ici et
        // se termine, pendant que la fenêtre existante est réaffichée.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            logger::log_file("Second instance detected, focusing main window");
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .manage(AppState {
            monitor_state: monitor_state.clone(),
            settings: settings.clone(),
        })
        .invoke_handler(tauri::generate_handler![
            list_adapters,
            enable_adapter,
            disable_adapter,
            test_connectivity,
            test_adapter_connectivity,
            set_routing_metric,
            configure_load_balancing,
            configure_failover,
            get_monitor_state,
            enable_auto_failover,
            disable_auto_failover,
            get_failover_state,
            get_settings,
            save_settings,
            get_logs,
            get_app_version,
            get_log_files,
            get_log_by_date,
            get_adapter_details,
            get_all_adapter_details,
        ])
        .setup(move |app| {
            // ── System Tray ──
            let show_i = MenuItemBuilder::with_id("show", "Afficher DualLink").build(app)?;
            let quit_i = MenuItemBuilder::with_id("quit", "Quitter").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_i)
                .separator()
                .item(&quit_i)
                .build()?;

            // Tray icon loaded from an embedded png file.
            // Dedicated 32x32 asset: `icon.png` is now 512x512 for the
            // installer, too heavy to downscale in the system tray.
            let icon_bytes = include_bytes!("../icons/tray-icon.png");
            let tray_icon = Image::from_bytes(icon_bytes).expect("Failed to load tray icon");

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .tooltip("DualLink - Gestionnaire réseau")
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.destroy();
                        }
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // ── Window close = hide to tray ──
            // X hides the window; only "Quitter" in tray menu exits.
            // The first hide shows an in-app notice before the window
            // disappears, so the user knows the app is still running and
            // where to find it (many don't look in the notification area).
            if let Some(window) = app.get_webview_window("main") {
                let w_clone = window.clone();
                let app_handle = app.handle().clone();
                let hide_notice_shown = Arc::new(std::sync::atomic::AtomicBool::new(false));

                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();

                        let first_hide =
                            !hide_notice_shown.swap(true, std::sync::atomic::Ordering::Relaxed);

                        if first_hide {
                            // Émet l'info, laisse le toast s'afficher, puis
                            // cache. 4s: le message nomme deux actions
                            // (« zone de notification », « clic droit →
                            // Quitter »), illisible en 1,5s.
                            let _ = app_handle.emit("hide-to-tray-notice", ());
                            let w = w_clone.clone();
                            tauri::async_runtime::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(4000)).await;
                                let _ = w.hide();
                            });
                        } else {
                            let _ = w_clone.hide();
                        }
                    }
                });
            }

            // ── Start background monitoring ──
            let handle = app.handle().clone();
            monitor::start_monitor(handle, monitor_state, settings);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running DualLink");
}
