# DualLink - Project Memory

## 📋 IMMUTABLE RULES
1. **MEMORY.md MUST be updated after every code change** — no exceptions, no shortcuts
2. **No regression** — every bug fix is documented and must never be reintroduced
3. **Read MEMORY.md first** at the start of every session before any action
4. **PHASE DE TEST EN COURS** — Pas de modifications de code sans accord explicite de l'utilisateur. Seule la mémoire peut être mise à jour.
5. **RELEASE CHECKLIST** — Avant chaque release : (a) Vérifier que README.md est à jour (features, architecture, backlog), (b) Vérifier que MEMORY.md reflète l'état actuel, (c) Nettoyer les fichiers temporaires, (d) Tester l'exe release

## 🎯 Project Overview
**DualLink** is a Windows network adapter manager built with **Tauri 2** (Rust backend + vanilla JS frontend).
Purpose: allow the user to toggle network adapters ON/OFF and mutualize them (load balancing or failover) for sharing a phone tethering connection alongside a slow Freebox ADSL.

## 📁 Project Structure
```
dualink/
├── package.json              # npm config, Vite + Tauri CLI
├── vite.config.js            # Vite dev server on port 1421
├── index.html                # Entry HTML
├── dist/                     # Built frontend (served by Tauri)
├── src/
│   ├── styles.css            # Glassmorphism dark theme (~minified)
│   ├── api.js                # Tauri invoke wrappers (19 commands)
│   ├── ui.js                 # DOM rendering, toggles, mode selector, toasts, dashboard, failover banner
│   └── app.js                # Entry point + monitoring-update listener
└── src-tauri/
    ├── Cargo.toml            # Rust deps: tauri 2, tokio, serde, anyhow, embed-resource
    ├── build.rs              # Embeds UAC manifest via embed-resource
    ├── tauri.conf.json       # App config, window 960x720, dark theme
    ├── app.manifest          # UAC manifest (requireAdministrator)
    ├── app.rc                # Resource file referencing app.manifest
    ├── capabilities/default.json  # Tauri permissions
    ├── icons/
    │   ├── icon.ico          # Windows app icon (32x32)
    │   └── icon.png          # Tray icon (32x32 blue globe, handcrafted)
    └── src/
        ├── main.rs           # Entry point (windows_subsystem = "windows")
        ├── lib.rs            # 19 Tauri commands + setup (tray, window, monitor)
        ├── logger.rs         # File logger to %LOCALAPPDATA%/DualLink/logs/ + panic hook
        ├── network.rs        # Native ping + PowerShell with CREATE_NO_WINDOW
        └── monitor.rs        # Background task: ping every 5s, emit events
```

## 🔧 Environment
- **OS**: Windows (Git Bash shell)
- **Node**: `C:/Program Files/nodejs/`
- **Cargo**: `C:/Users/endymion/.cargo/bin/`
- **PATH setup**: `export PATH="/c/Program Files/nodejs:/c/Users/endymion/.cargo/bin:$PATH"`
- **Project root**: `D:/Codex/dualink/`

## ✅ Build Commands
```bash
# Frontend build
cd dualink && npx vite build

# Rust check (fast, no output)
cd dualink/src-tauri && cargo check

# Release build
export PATH="/c/Program Files/nodejs:/c/Users/endymion/.cargo/bin:$PATH" && cd dualink/src-tauri && cargo build --release

# Run
dualink/src-tauri/target/release/duallink.exe

# Full build via Tauri CLI
cd dualink && npm run tauri:build
```

## 🐛 Bugs Fixed (DO NOT REGRESS)

1. **tokio::spawn panic** — `tokio::spawn()` was called outside a Tokio runtime. Fixed by using `tauri::async_runtime::spawn()`.
2. **Tray icon crash** — `Image::from_bytes()` does NOT support `.ico`. Fixed by using `.png` icon + `image-png` feature.
3. **Double window on tray click** — Removed redundant handler; Tauri 2 natively shows window on tray click.
4. **PowerShell windows popping** — Fixed by replacing `Test-Connection` with native `ping.exe` + `CREATE_NO_WINDOW` on ALL process spawns.
5. **No admin elevation** — Fixed by embedding `app.manifest` with `requireAdministrator` via `embed-resource`.
6. **Too many processes in monitoring** — 7-9 processes/5s. Fixed by removing per-adapter pings + caching. Result: ~1 process/5s.
7. **Ping latency always None on FR Windows** — `ping.exe` outputs `temps=`/`durée=` instead of `time=`. Fixed in `network.rs` parser.
8. **Expert mode slow + PS windows** — 5N+1 separate PS processes. Fixed by batching into single PS script.
9. **Monitor write lock blocking** — Failover held write lock during `set_routing_metric()`. Fixed: actions under lock, execute outside lock.

## 🧩 Architecture Decisions
- **No framework**: vanilla JS (no React/Vue) — lightweight, fast
- **CSS minified inline**: single `styles.css` with glassmorphism theme
- **Dashboard monitoring**: Canvas graph (DPR-aware) with last 10 latency measurements, gradient fill, threshold lines (50ms green, 100ms orange), live stats (current/avg/min/max/uptime). History stored in `latencyHistory[]` array in `ui.js`.
- **Native ping**: monitoring uses `ping.exe` via `run_hidden_command()` with `CREATE_NO_WINDOW` flag — zero visible windows
- **PowerShell (hidden)**: adapter management (list/enable/disable/metrics) uses `run_powershell()` which wraps `run_hidden_command()` with `CREATE_NO_WINDOW` flag
- **No visible process spawn**: ALL `Command::new()` calls go through `run_hidden_command()` which applies `creation_flags(CREATE_NO_WINDOW)` on Windows. Expert mode uses single batch PS script (no multiple process spawns).
- **Background monitoring**: `tauri::async_runtime::spawn` (NOT `tokio::spawn`) with configurable interval (default 5s, re-read each tick via RwLock). Adapter list cached (default 30s). No per-adapter ping. Uses `sleep` instead of `interval` for dynamic reconfiguration. Write lock released before network I/O (failover metrics swap) to avoid blocking the monitor loop and Tauri commands.
- **System tray**: X hides window to tray, "Quitter" exits + cleans lock file. Context menu: Afficher/Quitter
- **19 Tauri commands**: list_adapters, enable_adapter, disable_adapter, test_connectivity, test_adapter_connectivity, set_routing_metric, configure_load_balancing, configure_failover, get_monitor_state, enable_auto_failover, disable_auto_failover, get_failover_state, get_settings, save_settings, get_logs, get_log_files, get_log_by_date, get_adapter_details, get_all_adapter_details

## 🔬 Expert Mode Architecture
- **Tauri commands**: `get_adapter_details(name)`, `get_all_adapter_details()`
- **Backend**: `network.rs` exports `AdapterDetails` struct (IP, subnet, gateway, DNS, DHCP, metric, MAC, speed)
- **PowerShell**: Single batch script queries all adapters in one PS call (was 5N separate calls). Uses Get-NetAdapter + foreach loop with Get-NetIPAddress, Get-NetIPConfiguration, Get-DnsClientServerAddress, Get-NetIPInterface
- **Frontend**: New `expert` view in navigation — renders each adapter with detailed grid of network metrics
- **CSS**: `.expert-card`, `.expert-adapter`, `.expert-adapter-header`, `.expert-grid`, `.expert-field`, `.expert-label`, `.expert-value`, `.expert-ip`, `.expert-gw`, `.expert-status`

## 📊 Current Status (2026-08-22)
- **Build**: ✅ `cargo check` clean, `cargo build --release` OK
- **Exe**: ✅ 11.8 MB, launches without crash, stays open, tray icon visible, 0 errors
- **Window**: ✅ 960x720 (min 780x580) — fits without scrolling
- **Expert mode**: ✅ Single PS batch call, no PowerShell windows visible
- **UAC**: ✅ Exe requests admin on launch (requireAdministrator manifest embedded)
- **No PowerShell windows**: ✅ All process spawns use CREATE_NO_WINDOW + native ping.exe
- **Monitoring optimized**: ✅ ~1 process per tick (was 7-9). Adapter list cached. No per-adapter pings.
- **Auto failover**: ✅ Debounced (2 failures = 10s failover, 3 successes = 15s recovery). Banner UI.
- **Settings configurable**: ✅ Ping interval, target IP, adapter refresh — persisted in settings.json, UI tab.
- **Frontend**: ✅ Vite build OK, dist/ has CSS + JS assets
- **Code stats**: ~2200 lines total (1150 Rust + 700 JS + 180 CSS)
- **Single instance**: ✅ File lock with PID check prevents double launch
- **Log viewer**: ✅ Onglet Logs avec viewer scrollable, auto-refresh 3s, coloration par type, sélecteur date
- **Ready for testing**: ✅ No publish, no release — exe in target/release/
- **GitHub**: ✅ Public repo https://github.com/Endymi0n74/DualLink — v1.0.0 released with NSIS + MSI
- **README**: ✅ Documentation complète (features, architecture, build, backlog)

## ⏳ Backlog (après tests)
1. **Bundle NSIS** — activer `"active": true` dans `tauri.conf.json` pour installeur distribuable
2. **Notifications Windows** (toasts) quand le failover bascule
3. **Autostart Windows** — lancer DualLink au démarrage
4. **CSP (Content Security Policy)** — sécuriser le frontend pour production
5. ~~Mode expert~~ ✅ Implementé (2026-08-22) — onglet Expert avec IP/route/gateway/DNS/DHCP/métrique par adapter
6. **Tauri Updater** — auto-update depuis GitHub releases
7. **Export settings** — sauvegarder/charger un profil
8. **Historique métriques** — SQLite pour analyse long terme
9. **Multi-langue** (FR/EN)
10. **Crash reporting** — envoyer logs panic vers endpoint

## ⚠️ Known Issues / TODO
- **Bundle**: ✅ NSIS (2.6 MB) + MSI (3.8 MB) generated via `tauri build`. Language: FR (1036).
- **File logging**: ✅ `logger.rs` writes to `%LOCALAPPDATA%/DualLink/logs/YYYY-MM-DD.log`. Logs startup, all Tauri commands, monitor errors, and panics via custom hook. One file per day, append mode.
- **CSP is null**: Security CSP is disabled. Should add proper CSP for production.

## 🛡️ Auto-Failover Architecture
- **State**: `FailoverConfig` + `FailoverState` in `monitor.rs`, shared via `Arc<RwLock<MonitorState>>`
- **Lock pattern**: Read/write lock released before network I/O (set_routing_metric). Actions determined under lock, executed outside lock, state updated in single write lock.
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

## 📋 Log Viewer Architecture
- **Backend**: `logger.rs` exports `read_today_log()`, `list_log_files()`, `read_log_file(date)` — read from `%LOCALAPPDATA%/DualLink/logs/`
- **Commands**: `get_logs` (today), `get_log_files` (list all dates), `get_log_by_date(date)` (specific file)
- **Frontend**: Onglet `Logs` dans la nav — viewer scrollable `<pre>`, auto-refresh toutes les 3s, sélecteur de date, bouton pause/auto
- **Coloration**: `colorizeLogLine()` — ERROR/PANIC en rouge, FAILOVER en orange, CMD en bleu, Monitor en vert, séparateur en gris
- **CSS**: `.log-card`, `.log-header`, `.log-controls`, `.log-select`, `.log-legend`, `.log-content` (monospace, max-height 420px, scroll smooth), `.log-error/failover/cmd/monitor/separator/normal`

## 🔑 Conventions
- Rust: `anyhow::Result` for error handling, `serde` for serialization
- Frontend: vanilla JS ES modules, `import { invoke } from '@tauri-apps/api/core'`
- CSS: single file, CSS variables, no preprocessor
- Cargo package name: `duallink`
- Tauri config identifier: `com.endymi0n74.duallink`
