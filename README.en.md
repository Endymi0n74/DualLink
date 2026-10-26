# 🌐 DualLink

**Windows network card manager** — Share and pool your internet connections (phone + Box, for example) with smart automatic failover.

![Tauri](https://img.shields.io/badge/Tauri-2-blue) ![Rust](https://img.shields.io/badge/Rust-1.77-orange) ![License](https://img.shields.io/badge/License-MIT-green) ![Version](https://img.shields.io/badge/Version-1.3.3-brightgreen)

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
| 📱 **USB sharing (ADB)** | Plug the phone in: DualLink switches its USB function to `rndis` over ADB, the adapter shows up on its own |
| ⚖️ **Load Balancing** | Dynamic metrics to spread traffic |
| 🛡️ **Failover auto** | Loss detection (2 failures) → switch. Recovery (3 successes) → switch back |
| 📊 **Real-time dashboard** | Canvas latency chart, min/max/average stats, history of 10 measurements |
| 🔬 **Expert Mode** | IP, subnet mask, gateway, DNS, DHCP, routing metric per adapter |
| ⚙️ **Configurable settings** | Ping interval, target IP, adapter refresh, USB sharing (ADB) — persisted as JSON |
| 📋 **Log viewer** | Logs tab polling every 10 s, rendering only new lines, colouring by type, date picker |
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
- "USB sharing" button — switches the phone's USB function to `rndis` on demand

### Settings tab
- Ping interval (1-300s)
- Ping target (IP or hostname)
- Adapter refresh (5-600s)
- Automatic USB sharing (ADB), `adb.exe` path, ADB state test
- Save / Restore defaults

### Logs tab
- Scrollable viewer, polled every 10 s and only when the file grew
- Reading position kept while new lines arrive
- Colouring: 🔴 ERROR, 🟠 FAILOVER, 🔵 CMD, 🟢 Monitor
- Date picker

### Expert tab
- Network details per adapter: IP, subnet mask, gateway, DNS, DHCP, metric
- Refresh button

---
### Download the installer

Download from the [GitHub Releases](https://github.com/Endymi0n74/DualLink/releases):
- `DualLink_<version>_x64-setup.exe` — NSIS installer
- `DualLink_<version>_x64_en-US.msi` — MSI
- `DualLink_<version>_x64-portable.zip` — portable build

### Launch

```bash
# Double-click duallink.exe or the NSIS installer
```

> ⚠️ The application requests **admin rights** at launch (UAC) to manage network adapters.

---

## 📱 Phone USB sharing (ADB)

Plugging the phone in is enough: DualLink switches its USB function to `rndis`,
the phone starts sharing, and Windows gets a "Remote NDIS" adapter on its own.

**Requirements**
- **Android Platform-Tools** (`adb`) on the PC — auto-detected in `PATH`, winget
  and the Android SDK. The path can be forced in Settings.
- On the phone: Developer options → **USB debugging** on, and this PC
  authorised (accept the RSA key on the phone).

**What the app does**
- Checks `adb devices` every 10 s.
- If an authorised phone is plugged in and its USB function is not `rndis`, it
  runs `adb shell svc usb setFunctions rndis`.
- Sharing starts on the phone side (`rndis0 - TetheredState`), Windows gets a
  usable adapter, ready to become the failover secondary.

**Limits**
- The phone's Wi-Fi hotspot cannot be turned on remotely: Android refuses the
  shell command on consumer builds (HyperOS). The "Share screen" button opens
  the phone screen, the user flips the switch.
- Some models cut ADB when the USB function changes: replug the cable.

---

## 🛡️ Auto-Failover

```
Internet DOWN → 2 consecutive failures (10s) → auto switch to secondary
Internet UP   → 3 consecutive successes (15s) → restore primary
```

- **Debouncing** to prevent flapping
- **Tests the primary, not the secondary**: a `/32` host route is pinned to the primary (metric 1) before each probe, otherwise the ping would follow the default route through the secondary and the switch would oscillate
- Automatic metric swap (metric 10 → 100)
- UI banner with standby/active status + disable button
- Lock released before network calls — the monitor never blocks
- Disabling failover during a switch or quitting the app restores normal metrics

---

## 📈 Monitoring

- **Stable status**: two consecutive failures are needed to show "Disconnected",
  one success brings it back. Before the first probe the header shows
  "Checking…" instead of a wrong "Disconnected".
- **Default target `1.1.1.1`**: `8.8.8.8` lost half to all packets on some
  lines and made the header blink while the adapter was fine.
- **Only real adapters are probed**: virtual ones (Hyper-V, VMware, VPN) have no
  gateway to test, they show "Unknown state" instead of a false "Disconnected".
- **Every spawned process has a timeout** (PowerShell 20 s, ping/route 10 s,
  adb 15 s): a hung command can no longer freeze the monitor.

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
