use crate::logger;
use crate::network;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Emitter;
use tokio::sync::RwLock;

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
            ping_target: "8.8.8.8".to_string(),
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
        match std::fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
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
    std::env::var("LOCALAPPDATA")
        .ok()
        .map(|base| {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub enabled: bool,
    pub primary_index: u32,
    pub secondary_index: u32,
    pub primary_name: String,
    pub secondary_name: String,
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            primary_index: 0,
            secondary_index: 0,
            primary_name: String::new(),
            secondary_name: String::new(),
        }
    }
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
        let mut consecutive_failures: u32 = 0;
        let mut consecutive_successes: u32 = 0;
        const FAILOVER_THRESHOLD: u32 = 2;
        const RECOVERY_THRESHOLD: u32 = 3;

        loop {
            tick_count += 1;

            let (interval_secs, ping_target, adapter_refresh_secs) = {
                let s = settings.read().await;
                (s.interval_secs.max(1), s.ping_target.clone(), s.adapter_refresh_secs.max(5))
            };
            let refresh_interval = if interval_secs > 0 { adapter_refresh_secs / interval_secs } else { 6 };
            let refresh_interval = refresh_interval.max(1);

            let connectivity = network::test_connectivity_to(&ping_target).unwrap_or_else(|e| {
                logger::log_file(&format!("Monitor ping ERROR ({}): {}", ping_target, e));
                network::ConnectivityResult { reachable: false, latency_ms: None }
            });

            if tick_count % refresh_interval == 1 || cached_adapters.is_empty() {
                match network::list_adapters() {
                    Ok(adapters) => {
                        cached_adapters = adapters;
                        logger::log_file(&format!("Monitor adapters refreshed: {} found", cached_adapters.len()));
                    }
                    Err(e) => { logger::log_file(&format!("Monitor list_adapters ERROR: {}", e)); }
                }
            }

            let adapter_statuses: Vec<AdapterStatus> = cached_adapters
                .iter()
                .filter(|a| a.is_connected)
                .map(|a| AdapterStatus {
                    name: a.name.clone(),
                    reachable: connectivity.reachable,
                    latency_ms: connectivity.latency_ms,
                })
                .collect();

            // Auto-failover: determine actions under lock, execute outside lock
            let mut failover_action = String::new();
            let mut metrics_to_set: Vec<(u32, u32)> = Vec::new();
            {
                let mut state_guard = state.write().await;
                let fo = &mut state_guard.failover;

                if fo.config.enabled {
                    if !connectivity.reachable {
                        consecutive_failures += 1;
                        consecutive_successes = 0;
                        if consecutive_failures >= FAILOVER_THRESHOLD && !fo.is_failed_over {
                            logger::log_file(&format!(
                                "FAILOVER: switching to secondary '{}' ({}) after {} failures",
                                fo.config.secondary_name, fo.config.secondary_index, consecutive_failures
                            ));
                            metrics_to_set.push((fo.config.secondary_index, 10));
                            metrics_to_set.push((fo.config.primary_index, 100));
                            fo.is_failed_over = true;
                            fo.failover_count += 1;
                            fo.last_switch = chrono_now();
                            failover_action = format!("🔄 Failover → {}", fo.config.secondary_name);
                        }
                    } else {
                        consecutive_successes += 1;
                        consecutive_failures = 0;
                        if consecutive_successes >= RECOVERY_THRESHOLD && fo.is_failed_over {
                            logger::log_file(&format!(
                                "FAILOVER RECOVERY: restoring primary '{}' ({}) after {} successes",
                                fo.config.primary_name, fo.config.primary_index, consecutive_successes
                            ));
                            metrics_to_set.push((fo.config.primary_index, 10));
                            metrics_to_set.push((fo.config.secondary_index, 100));
                            fo.is_failed_over = false;
                            fo.last_switch = chrono_now();
                            failover_action = format!("✅ Restauré → {}", fo.config.primary_name);
                        }
                    }
                }
            }
            // Execute network calls WITHOUT holding any lock
            for (idx, metric) in &metrics_to_set {
                let _ = network::set_routing_metric(*idx, *metric);
            }

            let now = chrono_now();

            // Write state + build payload in one write lock
            let event_payload = {
                let mut state_guard = state.write().await;
                state_guard.internet_reachable = connectivity.reachable;
                state_guard.overall_latency_ms = connectivity.latency_ms;
                state_guard.adapters = adapter_statuses;
                state_guard.last_check = now.clone();

                let mut payload = MonitorState {
                    internet_reachable: state_guard.internet_reachable,
                    overall_latency_ms: state_guard.overall_latency_ms,
                    adapters: state_guard.adapters.clone(),
                    last_check: state_guard.last_check.clone(),
                    failover: state_guard.failover.clone(),
                };
                if !failover_action.is_empty() {
                    payload.last_check = format!("{} | {}", now, failover_action);
                }
                payload
            };

            let _ = app_handle.emit("monitoring-update", &event_payload);
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
        }
    });
}

fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hours = (now / 3600) % 24;
    let minutes = (now / 60) % 60;
    let seconds = now % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
