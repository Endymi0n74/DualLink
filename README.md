# 🌐 DualLink

**Gestionnaire de cartes réseau Windows** — Partagez et mutualisez vos connexions internet (téléphone + Freebox ADSL) avec un failover automatique intelligent.

![Tauri](https://img.shields.io/badge/Tauri-2-blue) ![Rust](https://img.shields.io/badge/Rust-1.77-orange) ![License](https://img.shields.io/badge/License-MIT-green)

---

## 🎯 Pourquoi DualLink ?

Vous avez une **Freebox ADSL** qui rame et un **partage de connexion téléphone** ? DualLink vous permet de :

- **Activer/désactiver** chaque carte réseau individuellement
- **Load Balancing** — répartir le trafic entre vos connexions
- **Failover automatique** — si une connexion tombe, bascule sur l'autre sans intervention
- **Monitorer** la latence en temps réel avec un graphique canvas

---

## ✨ Fonctionnalités

| Feature | Description |
|---------|-------------|
| 🔌 **Toggle adapters** | Activez/désactivez vos cartes réseau en un clic |
| ⚖️ **Load Balancing** | Métriques dynamiques pour répartir le trafic |
| 🛡️ **Failover auto** | Détection de perte (2 échecs) → bascule. Restauration (3 succès) → retour |
| 📊 **Dashboard temps réel** | Graphique canvas de latence, stats min/max/moyenne, historique 10 mesures |
| ⚙️ **Settings configurables** | Intervalle ping, cible IP, refresh adapters — persistés en JSON |
| 📋 **Log viewer** | Onglet logs avec auto-refresh, coloration par type, sélecteur date |
| 🔲 **System tray** | Minimise dans la barre des tâches au lieu de fermer |
| 🔒 **Admin auto** | Demande les droits admin au lancement (UAC) |
| 🚫 **Zero fenêtres** | Aucune fenêtre PowerShell visible — tout est en arrière-plan |

---

## 📸 Interface

### Onglet Accueil
- Liste des cartes réseau avec toggle ON/OFF
- Sélecteur de mode : Individuel / Load Balancing / Failover
- Dashboard monitoring avec graphique canvas et stats live
- Banner failover (standby/actif)

### Onglet Settings
- Intervalle de ping (1-300s)
- Cible ping (IP ou hostname)
- Refresh adapters (5-600s)
- Sauvegarder / Rétablir défauts

### Onglet Logs
- Viewer scrollable avec auto-refresh
- Coloration : 🔴 ERROR, 🟠 FAILOVER, 🔵 CMD, 🟢 Monitor
- Sélecteur de date

---

## 🏗️ Architecture

```
Frontend (Vanilla JS)          Backend (Rust / Tauri 2)
┌─────────────────────┐       ┌──────────────────────────┐
│  index.html         │       │  lib.rs                  │
│  src/styles.css     │◄─────►│  17 Tauri commands       │
│  src/api.js         │       │  monitor.rs (background) │
│  src/ui.js          │       │  network.rs (ping/PS)    │
│  src/app.js         │       │  logger.rs (file logs)   │
└─────────────────────┘       └──────────────────────────┘
                                      │
                              ┌───────▼───────┐
                              │  PowerShell   │
                              │  (hidden)     │
                              │  ping.exe     │
                              └───────────────┘
```

- **Frontend** : Vanilla JS (aucun framework), CSS glassmorphism dark theme
- **Backend** : Rust avec Tauri 2, async runtime Tokio
- **Monitoring** : ~1 process système par tick de 5s (optimisé)
- **Persistance** : settings.json + logs journaliers dans `%LOCALAPPDATA%/DualLink/`

---

## 🔧 Prérequis

- **Windows** 10/11 (64-bit)
- **Node.js** 18+
- **Rust** 1.77+ (via [rustup](https://rustup.rs/))
- **Tauri CLI** : `cargo install tauri-cli`

---

## 🚀 Installation

### Depuis les sources

```bash
# Cloner le repo
git clone https://github.com/Endymi0n74/DualLink.git
cd DualLink

# Installer les dépendances frontend
npm install

# Builder le frontend
npx vite build

# Builder l'exe release
cd src-tauri
cargo build --release
```

L'exe sera dans : `src-tauri/target/release/netmanager-lib.exe`

### Lancer

```bash
# Double-cliquer sur netmanager-lib.exe
# Ou depuis un terminal :
netmanager-lib.exe
```

> ⚠️ L'application demande les **droits admin** au lancement (UAC) pour gérer les cartes réseau.

---

## 📁 Structure du projet

```
DualLink/
├── index.html                # Point d'entrée HTML
├── package.json              # npm config
├── vite.config.js            # Config Vite (dev server port 1421)
├── src/
│   ├── styles.css            # Theme glassmorphism dark
│   ├── api.js                # Wrappers Tauri invoke
│   ├── ui.js                 # Rendu DOM, toggles, dashboard, failover
│   └── app.js                # Point d'entrée JS + listener monitoring
└── src-tauri/
    ├── Cargo.toml            # Deps Rust
    ├── build.rs              # Embed UAC manifest
    ├── tauri.conf.json       # Config Tauri
    ├── app.manifest          # Manifest UAC (requireAdministrator)
    ├── icons/
    │   ├── icon.ico          # Icône Windows
    │   └── icon.png          # Icône tray
    └── src/
        ├── main.rs           # Entry point
        ├── lib.rs            # Commands Tauri + setup
        ├── logger.rs         # Logger fichier + panic hook
        ├── network.rs        # Ping natif + PowerShell caché
        └── monitor.rs        # Monitoring background
```

---

## 📊 Process Budget

| Source | Avant | Après |
|--------|-------|-------|
| Ping global (8.8.8.8) | 1/5s | 1/5s |
| PowerShell list_adapters | 1/5s | 1/30s |
| Ping par adapter | N/5s | **0** (supprimé) |
| **Total (10 adapters)** | **~9 process/5s** | **~1 process/5s** |

---

## 🛡️ Auto-Failover

```
Internet DOWN → 2 échecs consécutifs (10s) → Bascule auto sur secondary
Internet UP   → 3 succès consécutifs (15s) → Restaure primary
```

- **Debouncing** anti-flapping
- Métriques swap automatique (metric 10 → 100)
- Banner UI avec statut standby/actif + bouton désactiver

---

## 📝 Logs

Les logs sont écrits dans : `%LOCALAPPDATA%/DualLink/logs/YYYY-MM-DD.log`

Format : `[HH:MM:SS] MESSAGE`

Types : ERROR (rouge), FAILOVER (orange), CMD (bleu), Monitor (vert)

---

## 🗺️ Backlog

- [ ] Bundle NSIS (installeur distribuable)
- [ ] Notifications Windows (toasts failover)
- [ ] Autostart Windows
- [ ] Mode expert (métriques IP/route/gateway)
- [ ] Tauri Updater (auto-update)
- [ ] Multi-langue (FR/EN)

---

## 📄 Licence

MIT — Fait avec ❤️ et Tauri 2

---

## 🙏 Credits

- [Tauri](https://tauri.app/) — Framework desktop Rust + JS
- [Vite](https://vitejs.dev/) — Build tool frontend
