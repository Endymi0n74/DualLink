# DualLink - Project Memory

## 📋 IMMUTABLE RULES
1. **MEMORY.md MUST be updated after every code change** — no exceptions, no shortcuts
2. **No regression** — every bug fix is documented and must never be reintroduced
3. **Read MEMORY.md first** at the start of every session before any action

## 🎯 Project Overview
**DualLink** is a Windows network adapter manager built with **Tauri 2** (Rust backend + vanilla JS frontend).
Purpose: allow the user to toggle network adapters ON/OFF and mutualize them (load balancing or failover) for sharing a phone tethering connection alongside a slow Freebox ADSL.

## 📁 Project Structure
```
netmanager/
├── package.json              # npm config, Vite + Tauri CLI
├── vite.config.js            # Vite dev server on port 1421
├── index.html                # Entry HTML
├── dist/                     # Built frontend (served by Tauri)
├── src/
│   ├── styles.css            # Glassmorphism dark theme (~minified)
│   ├── api.js                # Tauri invoke wrappers (12 commands)
│   ├── ui.js                 # DOM rendering, toggles, mode selector, toasts, dashboard, failover banner
│   └── app.js                # Entry point + monitoring-update listener
└── src-tauri/
    ├── Cargo.toml            # Rust deps: tauri 2, tokio, serde, anyhow, embed-resource
    ├── build.rs              # Embeds UAC manifest via embed-resource
    ├── tauri.conf.json       # App config, window 900x620, dark theme
    ├── app.manifest          # UAC manifest (requireAdministrator)
    ├── app.rc                # Resource file referencing app.manifest
    ├── capabilities/default.json  # Tauri permissions
    ├── icons/
    │   ├── icon.ico          # Windows app icon (32x32)
    │   └── icon.png          # Tray icon (32x32 blue globe, handcrafted)
    └── src/
        ├── main.rs           # Entry point (windows_subsystem = "windows")
        ├── lib.rs            # 9 Tauri commands + setup (tray, window, monitor)
        ├── logger.rs         # File logger to %LOCALAPPDATA%/DualLink/logs/ + panic hook
        ├── network.rs        # Native ping + PowerShell with CREATE_NO_WINDOW
        └── monitor.rs        # Background task: ping every 5s, emit events
```

## 🔧 Environment
- **OS**: Windows (Git Bash shell)
- **Node**: `C:/Program Files/nodejs/`
- **Cargo**: `C:/Users/endymion/.cargo/bin/`
- **PATH setup**: `export PATH="/c/Program Files/nodejs:/c/Users/endymion/.cargo/bin:$PATH"`
- **Project root**: `D:/Codex/netmanager/`

## ✅ Build Commands
```bash
# Frontend build
cd netmanager && npx vite build

# Rust check (fast, no output)
cd netmanager/src-tauri && cargo check

# Release build
export PATH="/c/Program Files/nodejs:/c/Users/endymion/.cargo/bin:$PATH" && cd netmanager/src-tauri && cargo build --release

# Run
netmanager/src-tauri/target/release/netmanager-lib.exe

# Full build via Tauri CLI
cd netmanager && npm run tauri:build
```

## 🐛 Bugs Fixed (DO NOT REGRESS)
1. **tokio::spawn panic** — `tokio::spawn()` was called outside a Tokio runtime in `monitor.rs`. Fixed by using `tauri::async_runtime::spawn()`.
2. **Tray icon crash** — `TrayIconBuilder::new()` without `.icon()` panics on Windows. Tauri 2 `Image::from_bytes()` does NOT support `.ico` format (gets `UnsupportedError { format: Exact(Ico) }`). Fixed by using a handcrafted `.png` icon and enabling `image-png` feature.
3. **Double window on tray click** — `on_tray_icon_event` handler was redundant with Tauri 2's default tray behavior. Removed the custom handler; Tauri 2 natively shows window on tray click.
4. **PowerShell windows popping** — Every 5s, monitoring spawned visible PowerShell windows for ping + adapter list. Fixed by: (a) replacing `Test-Connection` PowerShell with native `ping.exe` via `run_hidden_command()`, (b) adding `CREATE_NO_WINDOW` (0x08000000) flag via `CommandExt::creation_flags()` on ALL process spawns.
5. **No admin elevation** — Enable/Disable-NetAdapter require admin. Fixed by embedding `app.manifest` with `requireAdministrator` via `embed-resource` crate in `build.rs`. Build conflict with `winresource` (duplicate VERSION resource) was avoided by using `embed-resource` instead.
6. **Too many processes in monitoring** — Every 5s spawned: 1 global ping + 1 PowerShell (list_adapters) + N pings per connected adapter (5-8). Total: 7-9 processes per 5s tick. Fixed by: (a) removing redundant per-adapter pings — if internet is reachable, all connected adapters are working, (b) caching adapter list in monitor — refresh every 30s instead of every 5s. Result: ~1 process per 5s tick + 1 PowerShell every 30s.

## 🧩 Architecture Decisions
- **No framework**: vanilla JS (no React/Vue) — lightweight, fast
- **CSS minified inline**: single `styles.css` with glassmorphism theme
- **Dashboard monitoring**: Canvas graph (DPR-aware) with last 10 latency measurements, gradient fill, threshold lines (50ms green, 100ms orange), live stats (current/avg/min/max/uptime). History stored in `latencyHistory[]` array in `ui.js`.
- **Native ping**: monitoring uses `ping.exe` via `run_hidden_command()` with `CREATE_NO_WINDOW` flag — zero visible windows
- **PowerShell (hidden)**: adapter management (list/enable/disable/metrics) uses `run_powershell()` which wraps `run_hidden_command()` with `CREATE_NO_WINDOW` flag
- **No visible process spawn**: ALL `Command::new()` calls go through `run_hidden_command()` which applies `creation_flags(CREATE_NO_WINDOW)` on Windows
- **Background monitoring**: `tauri::async_runtime::spawn` (NOT `tokio::spawn`) with configurable interval (default 5s, re-read each tick via RwLock). Adapter list cached (default 30s). No per-adapter ping. Uses `sleep` instead of `interval` for dynamic reconfiguration.
- **System tray**: minimize to tray on X click, context menu "Afficher"/"Quitter", icon.png embedded
- **14 Tauri commands**: list_adapters, enable_adapter, disable_adapter, test_connectivity, test_adapter_connectivity, set_routing_metric, configure_load_balancing, configure_failover, get_monitor_state, enable_auto_failover, disable_auto_failover, get_failover_state, get_settings, save_settings

## 📊 Current Status (2026-08-22)
- **Build**: ✅ `cargo check` clean, `cargo build --release` OK
- **Exe**: ✅ 12 MB, launches without crash, stays open, tray icon visible, 0 errors
- **UAC**: ✅ Exe requests admin on launch (requireAdministrator manifest embedded)
- **No PowerShell windows**: ✅ All process spawns use CREATE_NO_WINDOW + native ping.exe
- **Monitoring optimized**: ✅ ~1 process per tick (was 7-9). Adapter list cached. No per-adapter pings.
- **Auto failover**: ✅ Debounced (2 failures = 10s failover, 3 successes = 15s recovery). Banner UI.
- **Settings configurable**: ✅ Ping interval, target IP, adapter refresh — persisted in settings.json, UI tab.
- **Frontend**: ✅ Vite build OK, dist/ has CSS + JS assets
- **Code stats**: 1536 lines total (874 Rust + 547 JS + 115 CSS)
- **Ready for testing**: ✅ No publish, no release — exe in target/release/

## ⚠️ Known Issues / TODO
- **Bundle disabled**: `"bundle": {"active": false}` in tauri.conf.json — no NSIS/MSI installer yet.
- **File logging**: ✅ `logger.rs` writes to `%LOCALAPPDATA%/DualLink/logs/YYYY-MM-DD.log`. Logs startup, all Tauri commands, monitor errors, and panics via custom hook. One file per day, append mode.
- **CSP is null**: Security CSP is disabled. Should add proper CSP for production.

## 🛡️ Auto-Failover Architecture
- **State**: `FailoverConfig` + `FailoverState` in `monitor.rs`, shared via `Arc<RwLock<MonitorState>>`
- **Debouncing**: 2 consecutive failures (10s) before failover, 3 consecutive successes (15s) before recovery — prevents flapping
- **Metrics swap**: On failover: secondary=metric 10 (priority), primary=metric 100. On recovery: restored.
- **Tauri commands**: `enable_auto_failover(primary_index, secondary_index, primary_name, secondary_name)`, `disable_auto_failover()`, `get_failover_state()`
- **MonitorState.failover** field carries the state to frontend via `monitoring-update` event
- **UI**: `.failover-banner` shows standby/active status with switch count and disable button
- **CSS**: `.failover-standby` (green tint), `.failover-active` (orange tint + pulse animation)

## 📊 Dashboard Monitoring (ui.js)
- **Canvas**: 500×120 logical px, DPR-aware, drawn via `drawDashboardCanvas()`
- **History**: `latencyHistory[]` array, max 10 entries, each `{ms, time, ok}`
- **Line graph**: Cyan (#00d4ff) line with gradient fill, dots at each measurement
- **Thresholds**: Dashed lines at 50ms (green) and 100ms (orange)
- **Stats grid**: 6 boxes — Current, Average, Min, Max, Uptime %, Measures count
- **CSS**: `.dashboard-card`, `.dashboard-grid`, `.dashboard-canvas-wrap`, `.dashboard-stats`, `.dash-stat`, `.dash-stat-label`, `.dash-stat-value`
- **Failover CSS**: `.failover-banner`, `.failover-standby`, `.failover-active`, `.failover-banner-icon/text/count`, `.btn-sm`, `.btn-danger`, `@keyframes pulse-border`, `@keyframes fade-in`

## 📐 Monitoring Process Budget
| Source | Before | After |
|--------|--------|-------|
| Global ping (8.8.8.8) | 1/5s | 1/5s |
| PowerShell list_adapters | 1/5s | 1/30s |
| Per-adapter ping (8.8.8.8) | N/5s (N=connected) | 0 (removed) |
| **Total (10 adapters)** | **~9 processes/5s** | **~1 process/5s** |

## ⚙️ Settings Architecture
- **Struct**: `Settings { interval_secs, ping_target, adapter_refresh_secs }` in `monitor.rs`
- **Persistence**: JSON file at `%LOCALAPPDATA%/DualLink/settings.json` — loaded on startup, saved on change
- **Defaults**: interval=5s, target=8.8.8.8, refresh=30s
- **Dynamic**: Monitor reads settings via `Arc<RwLock<Settings>>` each tick — interval changes take effect next tick
- **Validation**: interval 1-300s, refresh 5-600s, target any valid IP/hostname
- **Frontend**: Settings tab with 3 inputs (number/text) + save/defaults buttons + orange note about restart for interval
- **CSS**: `.settings-card`, `.settings-grid`, `.settings-group`, `.settings-input`, `.settings-hint`, `.settings-actions`, `.settings-note`, `.nav-btn`, `.header-nav`
- **Commands**: `get_settings`, `save_settings(interval_secs, ping_target, adapter_refresh_secs)`

## 🔑 Conventions
- Rust: `anyhow::Result` for error handling, `serde` for serialization
- Frontend: vanilla JS ES modules, `import { invoke } from '@tauri-apps/api/core'`
- CSS: single file, CSS variables, no preprocessor
- Cargo package name: `netmanager-lib` (NOT `duallink`)
- Tauri config identifier: `com.endymi0n74.duallink`
