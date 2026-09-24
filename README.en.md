# 🌐 DualLink

**Windows network card manager** — Share and pool your internet connections (phone + Box, for example) with smart automatic failover.

![Tauri](https://img.shields.io/badge/Tauri-2-blue) ![Rust](https://img.shields.io/badge/Rust-1.77-orange) ![License](https://img.shields.io/badge/License-MIT-green) ![Version](https://img.shields.io/badge/Version-1.0.0-brightgreen)

[🇫🇷 Français](README.md) · **🇬🇧 English**

---

## 🎯 Why DualLink?

Do you have a sluggish **ADSL Box** and a **shared phone connection**? DualLink lets you:

- **Enable/disable** each network adapter individually
- **Load Balancing** — spread traffic across your connections
- **Automatic Failover** — if a connection drops, switch to the other one with no manual intervention
- **Monitor** latency in real time with a canvas chart
- **Expert Mode** — view IP, gateway, DNS, DHCP, metric per adapter

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🔌 **Toggle adapters** | Enable/disable your network adapters in one click |
| ⚖️ **Load Balancing** | Dynamic metrics to spread traffic |
| 🛡️ **Failover auto** | Loss detection (2 failures) → switch. Recovery (3 successes) → switch back |
| 📊 **Real-time dashboard** | Canvas latency chart, min/max/average stats, history of 10 measurements |
| 🔬 **Expert Mode** | IP, subnet mask, gateway, DNS, DHCP, routing metric per adapter |
| ⚙️ **Configurable settings** | Ping interval, target IP, adapter refresh — persisted as JSON |
| 📋 **Log viewer** | Logs tab with auto-refresh, colouring by type, date picker |
| 🔲 **System tray** | Minimises to the taskbar instead of closing |
| 🔒 **Admin auto** | Requests admin rights at launch (UAC) |
| 🚫 **Zero windows** | No visible PowerShell window — everything runs in the background |
| 🔒 **Single instance** | Only one instance allowed — no duplicate tray icons |

---

## 📸 Interface

### Home tab
- List of network adapters with an ON/OFF toggle
- Mode selector: Individual / Load Balancing / Failover
- Monitoring dashboard with canvas chart and live stats
- Failover banner (standby/active)

### Settings tab
- Ping interval (1-300s)
- Ping target (IP or hostname)
- Adapter refresh (5-600s)
- Save / Restore defaults

### Logs tab
- Scrollable viewer with auto-refresh
- Colouring: 🔴 ERROR, 🟠 FAILOVER, 🔵 CMD, 🟢 Monitor
- Date picker

### Expert tab
- Network details per adapter: IP, subnet mask, gateway, DNS, DHCP, metric
- Refresh button

---
### Download the installer

Download from the [GitHub Releases](https://github.com/Endymi0n74/DualLink/releases).

### Launch

```bash
# Double-cliquer sur duallink.exe ou l'installeur NSIS
```

> ⚠️ The application requests **admin rights** at launch (UAC) to manage network adapters.

---

## 🛡️ Auto-Failover

```
Internet DOWN → 2 échecs consécutifs (10s) → Bascule auto sur secondary
Internet UP   → 3 succès consécutifs (15s) → Restaure primary
```

- **Debouncing** to prevent flapping
- Automatic metric swap (metric 10 → 100)
- UI banner with standby/active status + disable button
- Lock released before network calls — the monitor never blocks

---

## 📝 Logs

Logs are written to: `%LOCALAPPDATA%/DualLink/logs/YYYY-MM-DD.log`

Format: `[HH:MM:SS] MESSAGE`

Types: ERROR (red), FAILOVER (orange), CMD (blue), Monitor (green)

---

## 📄 License

MIT — Made with ❤️ and Tauri 2

---

## 🙏 Credits

- [Tauri](https://tauri.app/) — Rust + JS desktop framework
- [Vite](https://vitejs.dev/) — Frontend build tool
