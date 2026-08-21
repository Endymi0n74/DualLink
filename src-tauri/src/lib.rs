mod logger;
mod monitor;
mod network;

use std::sync::Arc;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager,
};
use tokio::sync::RwLock;

use monitor::{MonitorState, FailoverConfig, FailoverState, Settings};
use network::{ConnectivityResult, NetworkAdapter};

struct AppState {
    monitor_state: Arc<RwLock<MonitorState>>,
    settings: Arc<RwLock<Settings>>,
}

// ─── Tauri Commands ───────────────────────────────────────────────

#[tauri::command]
fn list_adapters() -> Result<Vec<NetworkAdapter>, String> {
    logger::log_file("CMD list_adapters");
    let result = network::list_adapters();
    match &result {
        Ok(adapters) => logger::log_file(&format!("CMD list_adapters: {} found", adapters.len())),
        Err(e) => logger::log_file(&format!("CMD list_adapters ERROR: {}", e)),
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn enable_adapter(name: String) -> Result<(), String> {
    logger::log_file(&format!("CMD enable_adapter: {}", name));
    let result = network::enable_adapter(&name);
    if let Err(ref e) = result {
        logger::log_file(&format!("CMD enable_adapter '{}' ERROR: {}", name, e));
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn disable_adapter(name: String) -> Result<(), String> {
    logger::log_file(&format!("CMD disable_adapter: {}", name));
    let result = network::disable_adapter(&name);
    if let Err(ref e) = result {
        logger::log_file(&format!("CMD disable_adapter '{}' ERROR: {}", name, e));
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn test_connectivity() -> Result<ConnectivityResult, String> {
    logger::log_file("CMD test_connectivity");
    let result = network::test_connectivity();
    if let Err(ref e) = result { logger::log_file(&format!("CMD test_connectivity ERROR: {}", e)); }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn test_adapter_connectivity(name: String) -> Result<ConnectivityResult, String> {
    logger::log_file(&format!("CMD test_adapter_connectivity: {}", name));
    let result = network::test_adapter_connectivity(&name);
    if let Err(ref e) = result { logger::log_file(&format!("CMD test_adapter_connectivity '{}' ERROR: {}", name, e)); }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn set_routing_metric(interface_index: u32, metric: u32) -> Result<(), String> {
    logger::log_file(&format!("CMD set_routing_metric: {} -> {}", interface_index, metric));
    let result = network::set_routing_metric(interface_index, metric);
    if let Err(ref e) = result { logger::log_file(&format!("CMD set_routing_metric ERROR: {}", e)); }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
fn configure_load_balancing(primary_index: u32, secondary_index: u32) -> Result<(), String> {
    logger::log_file(&format!("CMD configure_load_balancing: {} / {}", primary_index, secondary_index));
    network::configure_load_balancing(primary_index, secondary_index)
        .map_err(|e| { logger::log_file(&format!("CMD configure_load_balancing ERROR: {}", e)); e.to_string() })
}

#[tauri::command]
fn configure_failover(primary_index: u32, secondary_index: u32) -> Result<(), String> {
    logger::log_file(&format!("CMD configure_failover: {} / {}", primary_index, secondary_index));
    network::configure_failover(primary_index, secondary_index)
        .map_err(|e| { logger::log_file(&format!("CMD configure_failover ERROR: {}", e)); e.to_string() })
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
        "CMD enable_auto_failover: {}({}) -> {}({})",
        primary_name, primary_index, secondary_name, secondary_index
    ));
    // Set initial metrics: primary=10, secondary=100
    let _ = network::set_routing_metric(primary_index, 10);
    let _ = network::set_routing_metric(secondary_index, 100);
    let mut monitor = state.monitor_state.write().await;
    monitor.failover = FailoverState {
        config: FailoverConfig {
            enabled: true,
            primary_index,
            secondary_index,
            primary_name,
            secondary_name,
        },
        is_failed_over: false,
        failover_count: 0,
        last_switch: "Never".to_string(),
    };
    Ok(monitor.failover.clone())
}

#[tauri::command]
async fn disable_auto_failover(
    state: tauri::State<'_, AppState>,
) -> Result<FailoverState, String> {
    logger::log_file("CMD disable_auto_failover");
    let mut monitor = state.monitor_state.write().await;
    // Restore primary metrics if we were in failover
    if monitor.failover.is_failed_over {
        let _ = network::set_routing_metric(monitor.failover.config.primary_index, 10);
        let _ = network::set_routing_metric(monitor.failover.config.secondary_index, 100);
        logger::log_file("CMD disable_auto_failover: restored primary metrics");
    }
    monitor.failover.config.enabled = false;
    monitor.failover.is_failed_over = false;
    Ok(monitor.failover.clone())
}

#[tauri::command]
async fn get_failover_state(
    state: tauri::State<'_, AppState>,
) -> Result<FailoverState, String> {
    let monitor = state.monitor_state.read().await;
    Ok(monitor.failover.clone())
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> Result<Settings, String> {
    let s = state.settings.read().await;
    Ok(s.clone())
}

#[tauri::command]
async fn save_settings(
    state: tauri::State<'_, AppState>,
    interval_secs: u64,
    ping_target: String,
    adapter_refresh_secs: u64,
) -> Result<Settings, String> {
    logger::log_file(&format!(
        "CMD save_settings: interval={}s target={} refresh={}s",
        interval_secs, ping_target, adapter_refresh_secs
    ));
    let new_settings = Settings {
        interval_secs: interval_secs.max(1).min(300),
        ping_target,
        adapter_refresh_secs: adapter_refresh_secs.max(5).min(600),
    };
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
    let monitor_state = Arc::new(RwLock::new(MonitorState::default()));
    let settings = Arc::new(RwLock::new(Settings::load()));

    tauri::Builder::default()
        .manage(AppState {
            monitor_state: monitor_state.clone(),
            settings: settings.clone(),
        })
        .plugin(tauri_plugin_shell::init())            .invoke_handler(tauri::generate_handler![
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
        ])
        .setup(move |app| {
            // ── System Tray ──
            let show_i = MenuItemBuilder::with_id("show", "Afficher DualLink")
                .build(app)?;
            let quit_i = MenuItemBuilder::with_id("quit", "Quitter")
                .build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_i)
                .separator()
                .item(&quit_i)
                .build()?;

            // Load tray icon from embedded png file
            let icon_bytes = include_bytes!("../icons/icon.png");
            let tray_icon = Image::from_bytes(icon_bytes)
                .expect("Failed to load tray icon");

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
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // ── Window close = minimize to tray ──
            if let Some(window) = app.get_webview_window("main") {
                let w_clone = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w_clone.hide();
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
