# DualLink - Project Memory

## 📋 IMMUTABLE RULES
1. **MEMORY.md MUST be updated after every code change** — no exceptions, no shortcuts
2. **No regression** — every bug fix is documented and must never be reintroduced
3. **Read MEMORY.md first** at the start of every session before any action
4. **PHASE DE TEST EN COURS** — Pas de modifications de code sans accord explicite de l'utilisateur. Seule la mémoire peut être mise à jour.
5. **RELEASE CHECKLIST** — Avant chaque release : (a) Vérifier que README.md est à jour (features, architecture, backlog), (b) Vérifier que MEMORY.md reflète l'état actuel, (c) Nettoyer les fichiers temporaires, (d) Tester l'exe release

- When writing something intended for human consumption, (comment, commit message, reply to prompt) use as few words as possible. Pick every word meticulously to reduce the volume to a strict minimum. Be down to the point. Less is more.

- Avoid superlatives and praise. Stop telling me I am absolutely right. Give me the cold hard truth.

- Avoid magic numbers and strings by extracting recurring or meaningful values into descriptive constants (const) or enums. Keep self-explanatory, one-off values inline to avoid clutter. If a value comes from a spec (e.g. HTTP 200 OK), use a constant regardless.

- Reduce code indentation. Avoid Arrow Anti-Pattern. Leverage early return and continue.

- Keep function names short. Less than 30 characters.

- Use enums instead of booleans for function parameters.

- Let the reader of the code breathe. Add empty lines between logical blocks of code.

- Add a small, to the point, comment to explain *what* the block does and *why*. Use examples when possible. Propose ASCII drawings to explain complete systems.

- Treat member visibility changes as a breaking design shift. Keep all fields and functions private unless external access is strictly required by the design. Prompt the user for explicit approval before changing any access modifier from private to internal or public.

- Program to levels of abstraction. Lower-level mechanics (e.g., raw hardware I/O, sector parsing, direct socket streams) must be encapsulated in a dedicated driver/abstraction layer. Expose clean, high-level APIs to the rest of the application so calling code works with domain concepts, not raw implementation details.

- Don't touch blocks of code unrelated to the feature you implement. e.g. Don't add comments to a block of code if you did not create it or modify it. As much as possible try to minimize the number of changed lines when implementing a feature.

- Strictly adhere to the layered boundary hierarchy: each layer may only communicate with its immediate neighbor directly below it. Never "punch holes" through layers (e.g., controllers or UI components must never directly call database queries, raw hardware drivers, or low-level network clients; always route through the intermediate service/abstraction layer).

- Always use {}, even on a one-line "if" statement.

When you write a commit message, follow these 7 rules:
Rule 1: Separate the subject line from the body with a single blank line.
Rule 2: Limit the subject line to 50 characters (72 is the absolute hard limit).
Rule 3: Capitalize the first letter of the subject line.
Rule 4: Do not end the subject line with a period.
Rule 5: Use the imperative mood in the subject line (e.g., "Fix bug," "Add feature," 
        not "Fixed" or "Adds"). Test formula: It must complete the sentence: "If applied,
        this commit will [your subject line here]".
Rule 6: Wrap the body text manually at 72 characters to prevent Git formatting issues.
Rule 7: Use the body to explain what and why vs. how. Assume the code explains the how;
        the message must explain the context and reasoning. 

- If the prompt indicates that a bug is being fixed, don't write the fix right away. First write the test. Observe it failing. Then write the fix. And observe the test passing.        

## 🎯 Project Overview
**DualLink** is a Windows network adapter manager built with **Tauri 2** (Rust backend + vanilla JS frontend).
Purpose: allow the user to toggle network adapters ON/OFF and mutualize them (load balancing or failover) for sharing a phone tethering connection alongside a slow Freebox ADSL. Plugging the phone auto-enables its USB sharing adapter (RNDIS) with no click.

## 📁 Project Structure
```
dualink/
├── package.json              # npm config, Vite + Tauri CLI
├── vite.config.js            # Vite dev server on port 1421
├── index.html                # Entry HTML
├── dist/                     # Built frontend (served by Tauri)
├── src/
│   ├── styles.css            # Glassmorphism dark theme (~minified)
│   ├── api.js                # Tauri invoke wrappers (20 commands)
│   ├── ui.js                 # DOM rendering, toggles, mode selector, toasts, dashboard, failover banner
│   └── app.js                # Entry point + monitoring-update listener
└── src-tauri/
    ├── Cargo.toml            # Rust deps: tauri 2, chrono, tauri-plugin-single-instance, tokio, serde, anyhow
    ├── build.rs              # Embeds UAC manifest via embed-resource
    ├── tauri.conf.json       # App config, window 960x720, dark theme, version 1.2.0
    ├── app.manifest          # UAC manifest (requireAdministrator)
    ├── app.rc                # Resource file referencing app.manifest
    ├── capabilities/default.json  # Tauri permissions
    ├── icons/
    │   ├── icon.ico          # Icône Windows multi-tailles (16 -> 256)
    │   ├── icon.png          # Icône applicative 512x512 RGBA (installateur)
    │   └── tray-icon.png     # Icône barre des tâches 32x32 RGBA
    └── src/
        ├── main.rs           # Entry point (windows_subsystem = "windows")
        ├── lib.rs            # 20 Tauri commands + setup (tray, window, monitor)
        ├── logger.rs         # File logger to %LOCALAPPDATA%/DualLink/logs/ + panic hook
        ├── network.rs        # Native ping + PowerShell with CREATE_NO_WINDOW + pinned host routes
        └── monitor.rs        # Background task: ping every 5s, emit events, pure decide()
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
9. **Icon not RGBA for Linux/macOS** — icon.png was RGB. Fixed by converting to RGBA.
10. **Monitor write lock blocking** — Failover held write lock during `set_routing_metric()`. Fixed: actions under lock, execute outside lock.
11. **Failover en oscillation (flapping)** — Le ping de test passait par la route par défaut, donc par le secondaire (métrique passée à 10). Le secondaire répondait 3 fois, le primaire était restauré alors qu'il était toujours coupé, puis le failover se re-déclenchait. Fixed: route hôte `/32` épinglée sur le primaire (`RouteMetric 1`) avant chaque test — `ping -S` seul ne suffit pas. Fonction pure `decide()` + compteurs asymétriques (2 échecs / 3 succès) pour interdire l'oscillation. **NE PAS REVERTIR.**
12. **Métriques d'interface ignorées par Windows** — `Set-NetIPInterface -InterfaceMetric` sans `-AutomaticMetric Disabled`: Windows continuait d'appliquer la métrique automatique, donc le swap de métriques n'était qu'un no-op silencieux. Fixed: `-AutomaticMetric Disabled` systématique.
13. **Machine transformée en routeur** — `configure_load_balancing` activait `Set-NetIPInterface -Forwarding Enabled` sur TOUTES les interfaces. Fixed: supprimé (ce n'était pas l'effet recherché).
14. **Failover activable sans route.testable** — Si le primaire n'avait pas d'IP ni de passerelle, le failover s'activait quand même et testait la route par défaut. Fixed: `enable_auto_failover` refuse avec une erreur explicite.
15. **Nom de fichier de log faux + mauvaise date** — `format_timestamp` comptait 365 jours/an et 30 jours/mois, et UTC au lieu de l'heure locale. Fixed: `chrono::Local`, chemin recalculé à chaque écriture (bascule après minuit).
16. **Traversée de chemin via `get_log_by_date`** — `read_log_file("../../x")` lisait un fichier arbitraire. Fixed: `NaiveDate::parse_from_str` AVANT construction du chemin.
17. **Instance unique cassée au premier lancement** — Le lock file était écrit avant la création du dossier (`%LOCALAPPDATA%/DualLink/` inexistant) : échec silencieux. Et `is_process_alive` pouvait valider un PID réutilisé. Fixed: plugin officiel `tauri-plugin-single-instance`, lock file supprimé.
18. **Runtime async bloqué** — PowerShell et `ping.exe` étaient appelés directement dans des commandes `async` et dans la boucle du monitor. Fixed: `spawn_blocking` + gestion du `JoinError`.
19. **Statut par adaptateur trompeur** — Tous les adaptateurs affichaient le résultat d'un ping global. Fixed: ping par adaptateur via route épinglée, au rythme du refresh adapters.
20. **Masque de sous-réseau mal formaté** — `prefix_to_subnet(24)` renvoyait `255.255.255.0/24` pour un champ nommé `subnet_mask`. Fixed: `prefix_to_mask` renvoie le masque pointé seul.
21. **Ping réussi sans latence lisible** — Le test reposait sur `ttl=`/`durée=` et comparait en minuscules : `durée EXPIRÉ` ne pouvait jamais matcher. Fixed: succès = code de sortie 0 **ET** latence parsée, avec parsing FR/EN/cp850 testé.
22. **`enable_adapter` par nom** — Les noms d'adaptateurs sont localisés. Fixed: résolution nom → `-InterfaceIndex` (l'échappement reste nécessaire pour la résolution).
23. **Clamp de `Settings` non appliqué au chargement** — Un `settings.json` hors bornes n'était borné qu'à l'écriture. Fixed: `Settings::validated()` au chargement ET à la sauvegarde.
24. **ZIP portable mal nommé (CI)** — Le workflow codait en dur `DualLink_1.0.0_x64-portable.zip`. Depuis v1.0.1, chaque release publiait un ZIP portant le numéro de la version précédente. Fixed: nom dérivé du tag.
25. **Artefacts macOS/Linux inutilisables** — La CI publiait un .dmg, un .AppImage et un .deb alors que l'app ne peut pas fonctionner ailleurs : elle appelle `powershell.exe` et `ping.exe`, utilise 7 cmdlets Net* et un manifeste UAC `requireAdministrator`. Aucun fallback `cfg(not(windows))` n'existe. Ces trois artefacts (87 Mo) se construisaient et se téléchargeaient pour rien. Fixed: matrice CI limitée à Windows, artefacts morts supprimés de la v1.1.0.
26. **`Disable-NetAdapter` refuse `-InterfaceIndex`** — La consigne « utiliser `-InterfaceIndex` » a ete appliquee a tort : `Enable-NetAdapter` et `Disable-NetAdapter` n'acceptent QUE `-Name`, `-InterfaceDescription` ou `-InputObject`. Resultat : `FullyQualifiedErrorId: Windows System Error 5` sur chaque bascule de carte. Fixed: `Get-NetAdapter -Name` puis `-InputObject $if`. A verifier avant tout cmdlet: `Get-Command X | Select -ExpandProperty Parameters`.
27. **Tous les adaptateurs affiches « Deconnecte » alors que le reseau fonctionne** — `probe_routes` marquait `reachable: false` des que `New-NetRoute` echouait. Or cette cmdlet exige les droits admin: sans elevation, le Wi-Fi (IP + passerelle valides) sortait « Deconnecte » alors qu'il repondait en 17 ms. Fixed: repli sur `ping -S <ip_source>` quand la route epinglee est impossible. Le **failover reste strict**: decider sur une mesure degradee recreerait l'oscillation corrigee en #11.
28. **Le bouton X laisse croire a un plantage** — comportement voulu depuis le debut (fenetre masquee, seul « Quitter » du menu tray arrete), mais l'utilisateur cherche l'app dans la barre des taches, ne la trouve pas et conclut au crash. Fixed: au **premier** clic X, un toast violet explique ou se trouve l'icone et comment quitter; la fenetre se cache 4s plus tard (1,5s etait illisible). Les clics suivants masquent directement.
29. **Manifeste UAC ignoré : l'exe tournait sans élévation** — Le manifeste `requireAdministrator` n'a **jamais** été effectif : `embed-resource` + `app.rc` étaient écrasés par le manifeste par défaut de tauri-build (Common-Controls v6 seul, aucune `requestedExecutionLevel`), et `let _ =` masquait toute erreur d'embarquement. L'exe démarrait en `asInvoker` sans invite UAC (jeton filtré, `TokenElevation = 0`) : chaque `Disable-NetAdapter` échouait en « Accès refusé » / System Error 5, et la sortie PowerShell cp850 décodée en UTF-8 ajoutait du mojibak (« Acce□s refus□ »). La « session UAC refus uniquement » soupçonnée dans la section Limite était un contresens : aucune élévation n'était demandée. Fixed : `build.rs` appelle `tauri_build::try_build(Attributes::new().windows_attributes(WindowsAttributes::new().app_manifest(include_str!("app.manifest"))))`, `app.manifest` enrichi de la dépendance Common-Controls v6 (styles natifs préservés), `embed-resource`/`app.rc` supprimés. Les builds **release uniquement** portent l'élévation : un binaire de test/debug avec `requireAdministrator` refuse de démarrer hors shell élevé (os error 14001 / Permission denied), donc debug/test gardent le manifeste par défaut (`PROFILE == "release"` dans build.rs). Détection côté app : `network::is_elevated()` (TokenElevation via FFI, sans crate) vérifié avant chaque bascule → message actionnable au lieu d'une sortie PowerShell tronquée ; état tracé au démarrage (`Elevated: true/false`). Vérifié par inspection de la ressource RT_MANIFEST id 1 : `level=requireAdministrator`, fichier embarqué verbatim. **Piège SxS (bis, os error 14001)** : la première version corrigée refusait de démarrer (« configuration côte-à-côte incorrecte »). Deux fausses pistes éliminées avant la vraie cause, trouvée dans le **journal Événements Applications → source SideBySide, event 79** (l'outil de diagnostic recommandé par le message d'erreur lui-même — `sxstrace` exige une console interactive, inutilisable hors TTY) : « Le paramètre ...^dpiAware n'est pas inscrit ». **Chaque paramètre de windowsSettings doit être dans SON espace de noms enregistré : `dpiAware` → SMI/2005, `dpiAwareness` → SMI/2016** (pas 2011). Tout autre couplage fait rejeter tout le manifeste par CreateActCtx. Second piège avéré en chemin : tauri-winres, via `WindowsAttributes::app_manifest`, insère le manifeste dans un **bloc de chaînes rc** (`" {ligne} "` concaténées) — espaces ajoutés, accents convertis en `?` ; d'où l'inclusion verbatim du fichier via `new_without_app_manifest()` + `append_rc_content("1 24 \"app.manifest\"")` + `rerun-if-changed` déclaré à la main. Tests de non-régression : (1) `cmd /c exe` doit renvoyer 740 (privilège) ou 0 (démarré), **jamais 14001** ; (2) le log doit afficher `Elevated: true` au démarrage.

30. **Montage automatique du partage USB** (fonctionnalité v1.3.0) — Brancher le téléphone devait suffire à monter son partage USB, sans clic. Détection de l'adaptateur RNDIS via `network::is_usb_tethering(name, description)` (marqueurs FR/EN, insensible à la casse) et activation automatique dans la boucle du monitor dès que son état est `Disabled` (`STATUS_DISABLED`). Aucun processus supplémentaire: la détection réutilise le `list_adapters` déjà appelé au rythme du refresh; l'`Enable-NetAdapter` n'est lancé que lorsqu'un adaptateur désactivé apparaît. Garde anti-repétition (`tethering_to_enable` + `HashSet` des noms tentés): pas de relance PowerShell ni de WARN en boucle tant que l'adaptateur reste désactivé; le nom est oublié dès qu'il quitte l'état, donc un débranchement/rebranchement retente. Un adaptateur déjà monté (`Up`/`Disconnected`) est laissé tel quel. Événement `tethering-auto-enabled` → toast côté UI. **NE PAS REVERTIR.**

## 🧩 Architecture Decisions
- **No framework**: vanilla JS (no React/Vue) — lightweight, fast
- **CSS minified inline**: single `styles.css` with glassmorphism theme
- **Dashboard monitoring**: Canvas graph (DPR-aware) with last 10 latency measurements, gradient fill, threshold lines (50ms green, 100ms orange), live stats (current/avg/min/max/uptime). History stored in `latencyHistory[]` array in `ui.js`.
- **Native ping**: monitoring uses `ping.exe` via `run_hidden_command()` with `CREATE_NO_WINDOW` flag — zero visible windows
- **PowerShell (hidden)**: adapter management (list/enable/disable/metrics) uses `run_powershell()` which wraps `run_hidden_command()` with `CREATE_NO_WINDOW` flag
- **No visible process spawn**: ALL `Command::new()` calls go through `run_hidden_command()` which applies `creation_flags(CREATE_NO_WINDOW)` on Windows. Expert mode uses single batch PS script (no multiple process spawns).
- **Background monitoring**: `tauri::async_runtime::spawn` (NOT `tokio::spawn`) with configurable interval (default 5s, re-read each tick via RwLock). Adapter list cached (default 30s). Write lock released before network I/O (failover metrics swap) to avoid blocking the monitor loop and Tauri commands. All blocking calls (PowerShell, ping.exe) run in `tokio::task::spawn_blocking` with `JoinError` handling; a panicking worker thread is logged and the tick abandoned rather than propagated.
- **Ping parsing**: `parse_latency()` strips non-ASCII before matching keywords (`time`, `temps`, `dure`), because `ping.exe` emits the console code page (cp437/cp850 on FR Windows) and UTF-8 decoding turns accents into replacement chars. Success requires exit code 0 AND a parsed latency.
- **System tray**: X hides window to tray, "Quitter" exits. Context menu: Afficher/Quitter
- **20 Tauri commands**: list_adapters, enable_adapter, disable_adapter, test_connectivity, test_adapter_connectivity, set_routing_metric, configure_load_balancing, configure_failover, get_monitor_state, enable_auto_failover, disable_auto_failover, get_failover_state, get_settings, save_settings, get_logs, get_app_version, get_log_files, get_log_by_date, get_adapter_details, get_all_adapter_details

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
- **UAC**: ✅ Exe requests admin on launch — effectif depuis le fix du bug #29 (embarqué via `WindowsAttributes` de tauri-build; l'ancien embed-resource était écrasé par le manifeste par défaut)
- **No PowerShell windows**: ✅ All process spawns use CREATE_NO_WINDOW + native ping.exe
- **Monitoring optimized**: ✅ ~1 process per tick (was 7-9). Adapter list cached. Per-adapter status probed at adapter-refresh cadence.
- **Auto failover**: ✅ Debounced (2 failures = 10s failover, 3 successes = 15s recovery). Banner UI. **Primary tested via pinned host route — flapping fixed.**
- **Settings configurable**: ✅ Ping interval, target IP, adapter refresh — persisted in settings.json, UI tab, clamped on load and save
- **Frontend**: ✅ Vite build OK, dist/ has CSS + JS assets
- **Code stats**: ~2200 lines total (1150 Rust + 700 JS + 180 CSS)
- **Single instance**: ✅ Plugin `tauri-plugin-single-instance`, focuses main window on 2nd launch
- **Log viewer**: ✅ Onglet Logs avec viewer scrollable, auto-refresh 3s, coloration par type, sélecteur date
- **Quality gates**: ✅ `cargo check` / `clippy -D warnings` / `fmt --check` / 22 unit tests — all clean, CI gates on clippy
- **Version**: v1.2.3 publiée le 2026-10-14 (tag et release GitHub; artefacts NSIS, MSI et ZIP portable).
- **Icons**: ✅ Refaits: deux anneaux entrelacés (« dual links ») en dégradé cyan→violet. `icon.png` 512², `icon.ico` 9 tailles (16→256), `tray-icon.png` 32². Générés par `tools/gen_icons.py` (Pillow), version 1.1.0 du marque
- **Version affichée**: pastille `v1.2.3` dans l'en-tête, alimentée par la commande `get_app_version` (`CARGO_PKG_VERSION`) — jamais codée en dur dans le JS
- **GitHub**: ✅ Public repo https://github.com/Endymi0n74/DualLink — v1.1.0 released
- **Releases**: NSIS (Windows), MSI (Windows), ZIP portable (Windows) — artefacts macOS/Linux supprimés (bug #25)
- **CI/CD**: ✅ GitHub Actions, **Windows uniquement**, déclenché par tags v*, gardé par clippy -D warnings
- **README**: ✅ Documentation complète (features, architecture, build, backlog)

## 📊 Current Status (2026-10-14)
- **Build**: ✅ `cargo check`, `clippy -D warnings`, `fmt --check` et 27 tests unitaires propres
- **Version**: v1.2.3 (dernière release publiée; détails ci-dessous).
- **Failover**: ✅ Oscillation supprimée — le primaire est testé via une route hôte `/32` épinglée, pas via la route par défaut
- **Métriques**: ✅ `-AutomaticMetric Disabled` appliqué (sinon Windows ignorait `-InterfaceMetric`)
- **Forwarding**: ✅ Supprimé (la machine ne se comporte plus en routeur)
- **Logs**: ✅ `chrono::Local`, nouveau fichier après minuit, dates validées (pas de traversée de chemin)
- **Single instance**: ✅ Plugin officiel, lock file supprimé
- **Élévation**: ✅ Manifeste `requireAdministrator` effectif sur les builds release (bug #29) — ressource RT_MANIFEST id 1 vérifiée (`level=requireAdministrator`, espaces de noms 2005/2016); `cargo test` reste exécutable car debug/test gardent le manifeste par défaut
- **Bannière d'élévation**: ✅ Commande `get_elevation_status` + bandeau rouge persistant dans l'UI si l'app tourne sans droits admin (l'utilisateur voit le blocage AVANT de cliquer)
- **Backend renforcé (mission A-I, 27 tests)**: ✅ Nettoyage des routes `/32` orphelines au démarrage (PowerShell multi-interfaces, non bloquant); `reachable` = code de sortie du ping (langue-agnostique); champ `degraded` sur `ConnectivityResult` et `status_known` sur `AdapterStatus` (sondes `ping -S` ne simulent plus un adaptateur joignable); `ensure_elevated()` avant toute cmdlet; `disable_auto_failover` désactive TOUJOURS (restauration métriques échouée = WARN dans `last_switch`, pas d'erreur); pin/unpin de route via `route.exe` natif (~10 ms au lieu de 300-800 ms PowerShell) avec builders purs testés + `validate_gateway` IPv4 strict; WARN « passerelle inconnue » loggé une seule fois (flag réarmé); hook de panique en `try_lock` (pas de deadlock si WRITE_LOCK détenu); `is_valid_log_date` exige 10 caractères
- **Runtime**: ✅ Aucun appel PowerShell/ping sur le runtime async (`spawn_blocking`)
- **Statut par adaptateur**: ✅ Mesuré par adaptateur via route épinglée, rafraîchi au rythme du refresh adapters
- **Dépendances**: `+chrono`, `+tauri-plugin-single-instance`, `-tauri-plugin-shell`, `-windows-sys`
- **Testé sur cette machine**: Wi-Fi passe vert (17 ms), `disable_adapter` coupe réellement VMnet1 (vérifié: status `Disabled`), toast « masqué dans le tray » lisible avant fermeture (4s validé à l'usage); exe release 1.2.2 reconstruit depuis le tag et relancé: `Elevated: true`, zéro événement SideBySide (non-régression #29)
- **Limite**: Toute action nécessitant l'élévation échoue si le processus ne l'est pas. Depuis le fix #29, un lancement normal déclenche l'invite UAC; si un contexte lance l'exe sans élévation, l'app le détecte (`network::is_elevated()`, log `Elevated: false`) et renvoie un message clair (« Droits administrateur manquants… ») au lieu d'une sortie PowerShell. Le repli `ping -S` masque encore ce cas pour l'affichage des sondes.

## 📊 Current Status (2026-10-16)
- **Version**: v1.3.0 — montage automatique du partage USB du téléphone.
- **Fonctionnalité**: ✅ Adaptateur RNDIS détecté et activé seul quand le téléphone est branché (toujours actif, aucun réglage). Toast UI à l'activation.
- **Tests**: ✅ 31 unitaires (`cargo test`), dont 4 nouveaux: détection RNDIS EN/FR + marques, non-détection d'une carte normale, sélection/anti-repétition, retente après rebranchement.
- **Garde-fous**: ✅ `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings` propres; frontend `vite build` OK.
- **Périmètre**: aucun processus en plus en régime normal (réutilise `list_adapters`); `Enable-NetAdapter` uniquement quand un adaptateur désactivé apparaît.

## 📦 Release v1.3.0 (2026-10-16)
- Version synchronisée dans `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `Cargo.lock`, `tauri.conf.json`, `app.manifest` et badges README FR/EN.
- Nouvelle fonctionnalité: montage automatique du partage USB (RNDIS) au branchement du téléphone.
- README FR/EN: ligne « Montage USB auto » ajoutée.

## 📦 Release v1.2.3 (2026-10-14)
- Version synchronisée dans `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `Cargo.lock`, `tauri.conf.json` et `app.manifest`.
- Corrections failover : restauration des métriques nominales lors de la désactivation pendant la bascule et à la fermeture via le menu Quitter.
- README FR/EN et badges version mis à jour.
- Vérifications locales : frontend build, `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (27/27) et build Tauri release Windows réussis.
- Artefacts générés : NSIS 2,21 Mio et MSI 3,29 Mio. Le ZIP portable est généré par le workflow CI.
- Publication GitHub confirmée : [release v1.2.3](https://github.com/Endymi0n74/DualLink/releases/tag/v1.2.3), workflow CI réussi et trois artefacts téléchargeables.

## 🔧 Session de revue (non commitée)
- **Course failover / désactivation**: ✅ `disable_auto_failover` passe `enabled=false` et lit `is_failed_over` sous le MÊME verrou d'écriture; le monitor revérifie `enabled` sous verrou avant de publier une bascule et, si désactivé pendant l'application, remet les métriques nominales (sinon métriques permutées sans rien pour les restaurer)
- **Sortie (menu Quitter)**: ✅ `restore_nominal_metrics_if_failed_over` restaure les métriques avant `app.exit(0)` si un failover est actif
- **Doc**: `probe_primary` avait la doc du static `WARN_GATEWAY_MISSING` attachée; remise en place
- **Vérifié**: `cargo check` propre, `cargo test` 27/27. Non vérifié: comportement réel (bascule en cours + désactivation, sortie avec failover actif) — à tester sur la machine avec droits admin

## ⏳ Backlog (après tests)
1. ~~Bundle NSIS~~ ✅ Activé `"active": true` dans `tauri.conf.json` pour installeur distribuable
2. **Notifications Windows** (toasts) quand le failover bascule
3. **Autostart Windows** — lancer DualLink au démarrage
4. **CSP (Content Security Policy)** — sécuriser le frontend pour production
5. ~~Mode expert~~ ✅ Implementé (2026-08-22) — onglet Expert avec IP/route/gateway/DNS/DHCP/métrique par adapter
6. **Tauri Updater** — auto-update depuis GitHub releases
7. **Export settings** — sauvegarder/charger un profil
8. **Historique métriques** — SQLite pour analyse long terme
9. **Multi-langue** (FR/EN)
10. **Crash reporting** — envoyer logs panic vers endpoint
11. **Passerelle primaire figée** — `FailoverConfig.primary_gateway` est résolue à l'activation. Un changement de passerelle (reconnexion PPPoE, DHCP) invalide la route jusqu'à réactivation du failover
12. **Renommer `configure_load_balancing`** — nom trompeur (répartition de priorité, pas de load balancing). Conservée pour ne pas casser le frontend
13. ~~CI multiplateforme~~ ✅ Supprimé le 2026-10-11 — macOS et Linux ne sont plus construits (voir bug #25)

## ⚠️ Known Issues / TODO
- **Bundle**: ✅ NSIS + MSI + ZIP portable + DMG + AppImage + deb — all generated via `tauri build`. Language: FR (1036).
- **File logging**: ✅ `logger.rs` writes to `%LOCALAPPDATA%/DualLink/logs/YYYY-MM-DD.log` (heure locale, chemin recalculé à chaque écriture). Logs startup, all Tauri commands, monitor errors, and panics via custom hook. One file per day, append mode.
- **CSP is null**: Security CSP is disabled. Should add proper CSP for production.

## 🛡️ Auto-Failover Architecture
- **State**: `FailoverConfig` + `FailoverState` in `monitor.rs`, shared via `Arc<RwLock<MonitorState>>`
- **Lock pattern**: Read/write lock released before network I/O (set_routing_metric). Actions determined under lock, executed outside lock, state updated in single write lock.
- **Debouncing**: `decide()` is a PURE function: 2 consecutive failures (10s) before failover, 3 consecutive successes (15s) before recovery. Counters reset on every state change and on enable/disable.
- **Primary probe (fix for flapping)**: the failover decision uses `probe_primary()`, which pins a `/32` host route on the primary (`RouteMetric 1`, next hop = `primary_gateway`) before pinging. A plain ping would follow the default route — i.e. the secondary — and would falsely report the primary as recovered. `ping -S` alone is NOT sufficient.
- **Gateways**: `enable_auto_failover` resolves `primary_ip` + `primary_gateway` via `Get-NetIPConfiguration` and REFUSES activation (explicit `Err`) if either is missing — a failover without a pinnable route would test the wrong interface.
- **State update**: `is_failed_over` changes ONLY if every `set_routing_metric` succeeded. On failure: `ERROR` logged, counters re-armed to the threshold so the switch retries next tick.
- **Metrics swap**: On failover: secondary=metric 10 (priority), primary=metric 100. On recovery: restored. `-AutomaticMetric Disabled` is mandatory, else Windows ignores `-InterfaceMetric`.
- **Tauri commands**: `enable_auto_failover(primary_index, secondary_index, primary_name, secondary_name)`, `disable_auto_failover()`, `get_failover_state()`
- **MonitorState.failover** field carries the state to frontend via `monitoring-update` event
- **UI**: `.failover-banner` shows standby/active status with switch count and disable button
- **CSS**: `.failover-standby` (green tint), `.failover-active` (orange tint + pulse animation)

## 📱 Montage automatique du partage USB
- **Détection** (`network.rs`): `is_usb_tethering(name, description)` — vrai si le nom OU la description contient un marqueur `USB_TETHER_MARKERS` (rndis, ndis, tethering, internet sharing device, partage internet, apple mobile device ethernet, android, iphone), en minuscules. Couvre RNDIS EN/FR et les noms conviviaux de marque.
- **État ciblé**: `STATUS_DISABLED` (« Disabled »). Un adaptateur activé mais sans lien (« Disconnected ») ou déjà monté (« Up ») n'est jamais touché.
- **Décision** (`monitor.rs`): `tethering_to_enable(adapters, attempted)` — pure, testée. `attempted` (HashSet local à la boucle) empêche de retenter un nom tant qu'il reste désactivé; un nom quitte l'ensemble dès qu'il n'est plus dans l'état désactivé.
- **Exécution**: dans la boucle du monitor, sur la liste fraîche (au rythme du refresh adapters), via `spawn_blocking` → `network::enable_adapter`. Zéro processus en plus en régime normal.
- **UI**: événement Tauri `tethering-auto-enabled` (payload = noms activés) écouté par `app.js` → `showToast('📱 Partage USB monté : ...')`.
- **Pas de réglage**: comportement toujours actif (choix utilisateur).

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
| Primary probe (failover active) | 0 | 2/5s (pin + unpin) |
| Per-adapter ping (8.8.8.8) | N/5s (N=connected) | 3N/30s (at refresh cadence) |
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
