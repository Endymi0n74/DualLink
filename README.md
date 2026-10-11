# 🌐 DualLink

**Gestionnaire de cartes réseau Windows** — Partagez et mutualisez vos connexions internet (téléphone + Box par exemple) avec un failover automatique intelligent.

![Tauri](https://img.shields.io/badge/Tauri-2-blue) ![Rust](https://img.shields.io/badge/Rust-1.77-orange) ![License](https://img.shields.io/badge/License-MIT-green) ![Version](https://img.shields.io/badge/Version-1.2.1-brightgreen)

**🇫🇷 Français** · [🇬🇧 English](README.en.md)

---

## 🎯 Pourquoi DualLink ?

Vous avez une **Box ADSL** qui rame et un **partage de connexion téléphone** ? DualLink vous permet de :

- **Activer/désactiver** chaque carte réseau individuellement
- **Load Balancing** — répartition de priorité entre vos connexions (une destination ne suit qu’une seule interface, la plus basse métrique)
- **Failover automatique** — si une connexion tombe, bascule sur l'autre sans intervention
- **Monitorer** la latence en temps réel avec un graphique canvas
- **Mode Expert** — voir IP, passerelle, DNS, DHCP, métrique par adapter

---

## ✨ Fonctionnalités

| Feature | Description |
|---------|-------------|
| 🔌 **Toggle adapters** | Activez/désactivez vos cartes réseau en un clic |
| ⚖️ **Load Balancing** | Métriques dynamiques pour répartir le trafic |
| 🛡️ **Failover auto** | Détection de perte (2 échecs) → bascule. Restauration (3 succès) → retour |
| 📊 **Dashboard temps réel** | Graphique canvas de latence, stats min/max/moyenne, historique 10 mesures |
| 🔬 **Mode Expert** | IP, masque, passerelle, DNS, DHCP, métrique routing par adapter |
| ⚙️ **Settings configurables** | Intervalle ping, cible IP, refresh adapters — persistés en JSON |
| 📋 **Log viewer** | Onglet logs avec auto-refresh, coloration par type, sélecteur date |
| 🔲 **System tray** | Minimise dans la barre des tâches au lieu de fermer |
| 🔒 **Admin auto** | Demande les droits admin au lancement (UAC) |
| 🚫 **Zero fenêtres** | Aucune fenêtre PowerShell visible — tout est en arrière-plan |
| 🔒 **Single instance** | Plugin officiel Tauri — un seul exemplaire, fenêtre réaffichée au 2e lancement |
| 🔗 **Icônes sur mesure** | Deux anneaux entrelacés (les « liens » de DualLink), version affichée dans l’en-tête |

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

### Onglet Expert
- Détails réseau par adapter : IP, masque, passerelle, DNS, DHCP, métrique
- Bouton actualiser

---
### Télécharger l'installeur

Téléchargez depuis les [Releases GitHub](https://github.com/Endymi0n74/DualLink/releases) :
- `DualLink_<version>_x64-setup.exe` — installeur NSIS
- `DualLink_<version>_x64_en-US.msi` — MSI
- `DualLink_<version>_x64-portable.zip` — version portable

### Lancer

```bash
# Double-cliquer sur duallink.exe ou l'installeur NSIS
```

> ⚠️ L'application demande les **droits admin** au lancement (UAC) pour gérer les cartes réseau.
>
> Si votre session Windows est en UAC « refus uniquement » pour votre compte,
l'élévation peut échouer silencieusement. Lancez alors :
> `Start-Process ".\duallink.exe" -Verb RunAs`

---

## 🛡️ Auto-Failover

```
Internet DOWN → 2 échecs consécutifs (10s) → Bascule auto sur secondary
Internet UP   → 3 succès consécutifs (15s) → Restaure primary
```

- **Debouncing** anti-flapping
- **Test du primaire, pas du secondaire** : une route hôte `/32` est épinglée sur le primaire (métrique 1) avant chaque test, sans quoi le ping repartirait par le secondaire et provoquerait une oscillation
- Métriques swap automatique (metric 10 → 100), avec `-AutomaticMetric Disabled` (sans lui Windows ignore `-InterfaceMetric`)
- Banner UI avec statut standby/actif + bouton désactiver
- Lock libéré avant appels réseau — pas de blocage du monitor

---

## 📝 Logs

Les logs sont écrits dans : `%LOCALAPPDATA%/DualLink/logs/YYYY-MM-DD.log`

Format : `[HH:MM:SS] MESSAGE` (heure locale, nouveau fichier après minuit)

Types : ERROR (rouge), FAILOVER (orange), CMD (bleu), Monitor (vert)

---

## 📄 Licence

MIT — Fait avec ❤️ et Tauri 2

---

## 🙏 Credits

- [Tauri](https://tauri.app/) — Framework desktop Rust + JS
- [Vite](https://vitejs.dev/) — Build tool frontend
