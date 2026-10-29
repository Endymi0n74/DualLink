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

30. **Montage automatique du partage USB** (fonctionnalité v1.3.0, **retirée le 2026-10-20**) — voir « Montage automatique du partage USB »: Windows ne peut pas activer le partage USB côté téléphone, la fonctionnalité n'avait donc pas de déclencheur fiable. Conservé comme historique.

31. **Badge « Connecté » qui clignotait** — Un ping perdu isolé suffisait à afficher « Déconnecté ». Fixed: `Reachability` impose 2 échecs consécutifs pour perdre l'état et 1 succès pour le reprendre; `internet_known` affiche « Vérification… » avant la première mesure.

32. **Sondes inutiles et WARN en boucle** — Les cartes sans passerelle (Hyper-V, VMware, WSL, VPN) étaient sondées, avec un WARN de repli `ping -S` à chaque refresh. Fixed: `probe_candidates` exige une passerelle IPv4 valide; `Monitor adapters refreshed` n'est loggé que si `adapter_signature` change.

33. **Failover définitif sur le secondaire avec une cible en nom d'hôte** — La cible de ping peut être un nom d'hôte (réglage documenté), mais la sonde du primaire épingle une route `/32` via `route.exe ADD <cible> …`, qui n'accepte qu'une IPv4 numérique. L'échec de pose était traduit en « primaire injoignable » : bascule après 2 ticks, puis aucune restauration, chaque sonde échouant de la même façon. Fixed: `network::resolve_ipv4()` résout la cible à l'activation (`FailoverConfig.probe_target`), `enable_auto_failover` refuse l'activation sans adresse IPv4, et le nettoyage des routes orphelines au démarrage utilise la même adresse. Résolution unique: un nom DNS peut changer entre deux ticks et la route resterait épinglée sur une autre adresse.

34. **Toast du partage USB jamais affiché** — `app.js` appelait `showToast(...)` dans l'écouteur `usb-tethering-armed` sans l'importer de `ui.js`: le bundle gardait un appel à une globale inexistante (`ReferenceError` à chaque activation ADB, promesse rejetée, aucun toast). Fixed: `showToast` ajouté à l'import; vérifié par REA (`Unresolved call target showToast` présent avant, 0 après reconstruction).

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

## 🩹 Session du 2026-10-20 — stabilité monitoring et UI

- **Montage USB auto: fonctionnalité retirée** — Android décide seul d'exposer ou non une carte RNDIS; le PC ne peut pas déclencher le partage USB (pas de root/ADB). Téléphone branché + partage USB coupé = aucune interface réseau: la fonctionnalité n'avait pas de cas d'usage fiable. Supprimés: `is_usb_tethering`, `USB_TETHER_MARKERS`, `STATUS_DISABLED`, `tethering_to_enable`, `AdapterListAge`, `tether_suppressed`, l'activation auto dans le monitor, l'événement `tethering-auto-enabled`, le toast et la ligne README.
- **Badge « Connecté » qui repassait à « Déconnecté »** (bug #31): `Reachability` applique une hystérésis (2 échecs consécutifs pour perdre l'état, 1 succès pour le reprendre) et `internet_known` affiche « Vérification… » avant la première mesure.
- **Sondes inutiles et WARN en boucle** (bug #32): les adaptateurs sans passerelle valide (Hyper-V, VMware, WSL, VPN) ne sont plus sondés; `Monitor adapters refreshed` n'est loggé que si `adapter_signature` change.
- **Reconstruction complète de l'UI toutes les 5 s**: `patchLiveView()` remplace le rendu global — header, badges, dashboard et bandeau failover sont patchés sur place; l'historique du canvas est préservé.
- **Journal**: polling toutes les 5 s via `get_logs_if_changed` (taille connue), aucun transfert/rendu si le fichier n'a pas bougé; position de lecture et coloration conservées.
- **Erreurs PowerShell**: `first_error_line()` ne remonte que la première ligne utile (fin des pavés cp850).
- **Sélecteurs de mode**: les cartes virtuelles sont exclues du choix primaire/secondaire.
- **Canvas**: le DPR n'est plus appliqué en boucle (le graphe ne grossit plus à chaque redraw en 125/150 %).
- **Cible de connectivité par défaut** — Le badge d'en-tête oscillait alors que le Wi-Fi répondait: `8.8.8.8` perdait 50 à 100 % des paquets sur cette ligne (mesuré), tandis que `1.1.1.1` répondait à 16 ms sans perte. `DEFAULT_PING_TARGET` passe à `1.1.1.1`; valeurs par défaut, placeholder et bouton « Défauts » de l'UI alignés. Vérifié: 90 s sans bascule après redémarrage, contre 221 événements sur la session précédente.
- **Liste des cartes vivante** — `MonitorState.adapter_names` publie tous les noms vus par le monitor; l'UI compare (ordre ignoré) et recharge `list_adapters` une fois quand le matériel change (téléphone branché, carte ajoutée/retirée), sans attendre « Actualiser ».
- **Journal**: la position de lecture est restaurée explicitement si l'utilisateur n'était pas en bas; une erreur de lecture remet `logLength` à zéro pour retenter au tick suivant au lieu de figer le message.
- **Tests**: 36 unitaires (`cargo test`), `cargo fmt --check`, `cargo clippy -D warnings` et `npm run build` propres.
- **Vérifié en réel** (exe release relancé, PID 17504): une seule ligne `Monitor adapters refreshed`, zéro WARN, aucun pavé PowerShell sur 80 s. L'adaptateur RNDIS n'apparaissait pas: le partage USB côté téléphone était coupé.

## 📊 Current Status (2026-10-16)
- **Version**: v1.3.0 — montage automatique du partage USB du téléphone.
- **Fonctionnalité**: ✅ Adaptateur RNDIS détecté et activé seul quand le téléphone est branché (toujours actif, aucun réglage). Toast UI à l'activation.
- **Tests**: ✅ 31 unitaires (`cargo test`), dont 4 nouveaux: détection RNDIS EN/FR + marques, non-détection d'une carte normale, sélection/anti-repétition, retente après rebranchement.
- **Garde-fous**: ✅ `cargo fmt --check`, `cargo check`, `cargo clippy --all-targets -- -D warnings` propres; frontend `vite build` OK.
- **Périmètre**: aucun processus en plus en régime normal (réutilise `list_adapters`); `Enable-NetAdapter` uniquement quand un adaptateur désactivé apparaît.

## 📱 Partage USB du téléphone via ADB (2026-10-20)
- **Constat**: Android n'expose la carte RNDIS que si la fonction USB passe en `rndis`. Windows ne peut pas le décider seul; le téléphone, si. Testé sur POCO X7 (HyperOS): `adb shell svc usb setFunctions rndis` fonctionne, le téléphone démarre son partage (`rndis0 - TetheredState`) et Windows voit la carte RNDIS. L'état ne survit PAS au débranchement (mesuré: `sys.usb.config` revient à `adb`), donc la commande doit être rejouée à chaque branchement.
- **Module** (`adb.rs`): `locate()` (chemin configuré, PATH, winget, SDK Android), `parse_devices()`, `plan()` (premier appareil autorisé), `usb_function()` (`getprop sys.usb.config`), `has_rndis()`, `set_rndis()`, `open_tethering_screen()` (`am start -a android.settings.TETHER_SETTINGS`). Processus sans fenêtre, délai maximal 15 s, `CREATE_NO_WINDOW`.
- **Piège**: `svc` renvoie le code 255 même quand la commande passe. Le succès se lit dans la sortie (`setCurrentFunctions opId:1`), pas dans le code de retour.
- **Décision côté téléphone**: la fonction USB demandée au téléphone fait foi, pas la liste des cartes Windows (elle a jusqu'à 30 s de retard). `Adb::check()` renvoie `Present` / `NoDevice` / `Armed` / `Failed`.
- **Cadence**: une tentative toutes les 10 s, dans un thread détaché: une commande ADB lente ne retarde pas le cycle de monitoring. Le résultat revient par un drapeau lu au tick suivant (journal + toast + relecture de la liste).
- **Point d'accès Wi-Fi**: impossible à allumer à distance sur HyperOS (`cmd wifi set-wifi-ap-enabled` → SecurityException; `cmd tethering` → « No shell command implementation »). Seul l'écran de partage peut être ouvert sur le téléphone, l'utilisateur bascule l'interrupteur. Sans importance pour le PC, qui passe par le câble.
- **Délais sur tous les processus** (`network.rs`): `run_hidden_command` a désormais un délai maximal (20 s PowerShell, 10 s ping/route) et tue le processus. `Command::output()` attendait indéfiniment: un `Get-NetAdapter` pendant une ré-enumération USB bloquait le monitor (constaté: 2 min sans journal).
- **Réglages**: `usb_tether_auto` (défaut vrai) et `adb_path` (vide = détection). Bouton « Partage USB », « Tester ADB », « Écran de partage » dans l'UI.
- **Tests**: 41 unitaires (parsing `adb devices`, choix de l'appareil, détection RNDIS, réglages ADB, backward compatible du `settings.json`).

## 🩹 Session du 2026-10-26 — failover, réglages et toast USB
- **Bug #33 corrigé**: cible de ping en nom d'hôte → `route.exe ADD` refuse le nom, la sonde du primaire échouait à chaque tick et le failover basculait définitivement sur le secondaire. `probe_primary` utilise désormais `probe_target` (IPv4 résolue à l'activation); `probe_host()` retombe sur la cible du tick si le champ est vide (configuration héritée).
- **Réglage changé pendant un failover actif**: la cible résolue reste celle de l'activation, comme `primary_gateway` (backlog 11). Réactiver le failover pour en prendre une nouvelle.
- **Bug #34 corrigé**: le toast du partage USB ne s'affichait jamais — `showToast` manquait dans l'import de `app.js`, l'appel restait une globale inexistante. Détecté par `rea analyze-javascript-application dist` (`Unresolved call target showToast`); après correctif et rebuild, plus aucune référence non résolue et l'appel pointe sur le binding importé.
- **Réglages**: `settings.json` écrit dans `settings.json.tmp` puis renommé (`write_atomic`): un arrêt pendant l'écriture ne laisse plus un JSON tronqué, que `Settings::load` remplaçait silencieusement par les défauts. Test de non-régression: `sauvegarde_atomique_sans_residu`.
- **Tests**: 44 unitaires (`cargo test`), `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` et `npm run build` propres. Non vérifié en réel: pose d'une route `/32` vers une IPv4 résolue (session sans droits admin).

## 🔧 CI — runtime des actions (2026-10-29)
- **Avertissement `Node.js 20 is deprecated`**: il ne vient pas de l'entrée `node-version` de `setup-node`, mais du **runtime des actions elles-mêmes** (`runs.using` dans leur `action.yml`). Changer `node-version: 20` en `24` ne l'aurait donc pas supprimé.
- Actions réellement signalées par le run 40872170968: `actions/checkout@v4`, `actions/setup-node@v4`, `actions/upload-artifact@v4` (job build) et `actions/download-artifact@v4`, `softprops/action-gh-release@v2` (job release). `swatinem/rust-cache@v2` et `tauri-apps/tauri-action@v0` ne l'étaient pas.
- Bump vers les versions `node24`: `checkout@v7`, `setup-node@v7`, `upload-artifact@v7`, `download-artifact@v8`, `action-gh-release@v3`. Vérifié dans chaque `action.yml` (runtime + entrées utilisées: `merge-multiple`, `if-no-files-found`, `files`, `body`) avant de changer, pour ne pas casser l'upload/download des artefacts.
- Entrée `node-version` passée à `24` (Node 20 EOL depuis avril 2026; `vite@5.4.21` déclare `^18 || >=20`, `@tauri-apps/cli@2.12.2` `>=10` — les deux acceptent 24).
- **Vérifié par le run 40898795301** (`workflow_dispatch` sur `master`, commit `3c60128`): build Windows réussi en 6 min 07, **zéro annotation** remontée par l'API `check-runs/*/annotations`, et aucune occurrence de `Node.js 20` ni `deprecat` dans les 1332 lignes de log. `setup-node@v7` a installé `node v24.22.1`.
- **Méthode de mesure**: le même endpoint, appliqué au run 40872170968, remonte bien les deux `warning: Node.js 20 is deprecated` — le zéro sur le nouveau run est donc une absence réelle, pas un endpoint muet. Toujours comparer avant/après avec la même requête.
- **Tester la CI sans publier de release**: `gh workflow run release.yml --ref master -f tag=...`. Sur une branche, le job `release` est sauté par son `if: startsWith(github.ref, 'refs/tags/v')` — les actions du job release (`download-artifact`, `action-gh-release`) ne sont donc **pas** exercées; seul leur `action.yml` garantit `node24`.
- **Confirmé sur un vrai run de release** (run 40902420943, tag v1.3.5): les cinq actions ont tourné, `##[group]Run actions/download-artifact@v8` compris, et `Node.js 20` compte **2 occurrences** dans le log du run de la v1.3.4 contre **0** dans celui de la v1.3.5.
- **Faux positif à connaître**: `grep -i deprecat` sur les logs remonte un `[DEP0005] DeprecationWarning: Buffer() is deprecated` émis **dans** `download-artifact`. Il était déjà là en `v4` (même run 40872170968, `release/UNKNOWN STEP`): il vient de l'action, pas du workflow. Ne pas le confondre avec l'avertissement Node 20, qui lui est une annotation du runner.
- Reste cosmétique, hors périmètre: l'avertissement `ubuntu-latest will migrate to Ubuntu 26` (notice, pas warning) subsiste sur le job release.

## 📦 Release v1.3.5 (2026-10-29)
- **Release d'outillage**: aucun changement de code, binaire fonctionnellement identique à la v1.3.4. Sert à ce que le correctif node24 du workflow arrive dans un build réellement publié.
- Version synchronisée dans les six fichiers de métadonnées (`app.manifest`: `1.3.5.0`) + badges README FR/EN; entrée `1.3.5` en tête de `CHANGELOG.md`.
- Vérifications avant tag: 44 tests, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` (compile bien `duallink v1.3.5`), `npm run build` (27,60 kB).
- CI run 40902420943 réussie (build 3 min 05, release 16 s), trois artefacts: NSIS 2,36 Mio, MSI 3,52 Mio, ZIP portable 3,10 Mio. Publiée: [release v1.3.5](https://github.com/Endymi0n74/DualLink/releases/tag/v1.3.5), marquée *Latest*.
- Reste: la *notice* `ubuntu-latest will migrate to Ubuntu 26` (information runner, pas un avertissement du projet).

## 📦 Release v1.3.4 (2026-10-29)
- **Release de documentation**: aucun changement de code depuis la v1.3.3, binaires fonctionnellement identiques. Publiée à la demande explicite de l'utilisateur, pour disposer d'un journal des versions et d'un seul téléchargement par plateforme.
- `CHANGELOG.md` créé (v1.2.0 → v1.3.4, sections Ajouté/Corrigé/Retiré/Modifié); README FR/EN: badge 1.3.4 et lien `CHANGELOG.md`.
- Version synchronisée dans `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `Cargo.lock`, `tauri.conf.json` et `app.manifest` (`1.3.4.0`).
- **Anciennes releases supprimées**: `gh release delete v1.3.2|v1.3.3 --cleanup-tag --yes` puis `git tag -d v1.3.2 v1.3.3`. `git ls-remote --tags` ne liste plus que `v1.3.4` : un clone ancien doit refetcher les tags (`git fetch --prune --tags`), sinon `v1.3.3` semble encore local.
- **Piège du corps de release**: un lien relatif (`../blob/master/CHANGELOG.md`) ne résout pas dans une release GitHub. Utiliser l'URL absolue `https://github.com/Endymi0n74/DualLink/blob/master/CHANGELOG.md` (corrigé dans `release.yml` et dans la release publiée).
- Vérifications: 44 tests, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `npm run build` (27,60 kB).
- Publication GitHub confirmée : [release v1.3.4](https://github.com/Endymi0n74/DualLink/releases/tag/v1.3.4), CI réussie (run 40872170968), trois artefacts : NSIS 2,36 Mio, MSI 3,52 Mio, ZIP portable 3,10 Mio.

## 📦 Release v1.3.3 (2026-10-26)
- **Toast du partage USB**: `showToast` manquait dans l'import de `app.js`; le bundle gardait un appel à une globale inexistante (`ReferenceError` à chaque activation ADB). Détecté par `rea analyze-javascript-application dist`, disparu après reconstruction.
- **Sonde du failover**: cible de ping en nom d'hôte → IPv4 résolue à l'activation (`probe_target`); `route.exe` n'accepte pas un nom, la sonde échouait et le failover basculait définitivement sur le secondaire.
- **Réglages**: `settings.json` écrit dans `settings.json.tmp` puis renommé, au lieu d'être tronqué en place (JSON incomplet = retour silencieux aux défauts).
- Version synchronisée dans `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `Cargo.lock`, `tauri.conf.json`, `app.manifest` et badges README FR/EN.
- Vérifications: 44 tests, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `npm run build`, build release Windows (MSI 3,35 Mio, NSIS 2,25 Mio).
- Publication GitHub confirmée : [release v1.3.3](https://github.com/Endymi0n74/DualLink/releases/tag/v1.3.3), CI réussie (run 40287261678), trois artefacts : NSIS 2,25 Mio, MSI 3,36 Mio, ZIP portable 2,96 Mio.

## 📦 Release v1.3.2 (2026-10-20)
- Partage USB du téléphone piloté par ADB: au branchement, la fonction USB du téléphone passe en `rndis` (10 s) et la carte RNDIS apparaît seule.
- Délais maximaux sur tous les processus lancés (PowerShell 20 s, ping/route 10 s, adb 15 s): `Command::output()` attendait indéfiniment et pouvait geler le monitor.
- Version synchronisée dans `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `Cargo.lock`, `tauri.conf.json`, `app.manifest` et badge README.
- Vérifications: 41 tests, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `npm run build`, build release Windows, cycle branchement testé sur POCO X7 (partage activé, ping 0 % de perte).
- README FR et EN remis à niveau (v1.3.2): section « Partage USB du téléphone (ADB) » avec prérequis et limites, section « Monitoring », onglets Interface et Logs à jour.
- Publication GitHub confirmée : [release v1.3.2](https://github.com/Endymi0n74/DualLink/releases/tag/v1.3.2), CI réussie (run 39487390914), trois artefacts : NSIS 2,35 Mio, MSI 3,51 Mio, ZIP portable 3,10 Mio. Notes de release détaillées (prérequis ADB, limites).

## 📦 Release v1.3.1 (2026-10-20)
- Retrait du montage USB automatique: Android seul décide d'exposer une carte RNDIS, le PC ne peut pas déclencher le partage.
- Cible de connectivité par défaut passée à `1.1.1.1`: `8.8.8.8` perdait 50 à 100 % des paquets sur la ligne testée et faisait clignoter le badge.
- Badge avec hystérésis, sondes limitées aux cartes avec passerelle, liste des cartes rafraîchie à chaud, journal sans rendu inutile (polling 10 s, suspendu fenêtre masquée).
- Version synchronisée dans `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `Cargo.lock`, `tauri.conf.json`, `app.manifest` et badge README.
- Vérifications: 36 tests, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `npm run build`, build release Windows.
- Publication GitHub confirmée : [release v1.3.1](https://github.com/Endymi0n74/DualLink/releases/tag/v1.3.1), CI réussie (run 39478984625), trois artefacts : NSIS 2,32 Mio, MSI 3,46 Mio, ZIP portable 3,05 Mio. Notes de release détaillées (corrections + retrait du montage USB).

## 📦 Release v1.3.0 (2026-10-16)
- Version synchronisée dans `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `Cargo.lock`, `tauri.conf.json`, `app.manifest` et badges README FR/EN.
- Nouvelle fonctionnalité: montage automatique du partage USB (RNDIS) au branchement du téléphone.
- README FR/EN: ligne « Montage USB auto » ajoutée.
- Artefacts locaux vérifiés : `duallink.exe` release, MSI 3,29 Mio et NSIS 2,21 Mio pour 1.3.0.
- Publication GitHub confirmée : [release v1.3.0](https://github.com/Endymi0n74/DualLink/releases/tag/v1.3.0), CI réussie, trois artefacts (NSIS, MSI, ZIP portable).

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
11. **Passerelle primaire figée** — `FailoverConfig.primary_gateway` est résolue à l'activation. Un changement de passerelle (reconnexion PPPoE, DHCP) invalide la route jusqu'à réactivation du failover. Même chose pour `probe_target` (IPv4 de la cible de ping) : changer la cible pendant un failover actif n'a d'effet qu'à la réactivation
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

## 📱 Montage automatique du partage USB (retiré le 2026-10-20)
- **Raison**: Android décide seul d'exposer une carte RNDIS; le PC ne peut pas activer le partage USB. Téléphone branché avec partage coupé = aucune interface réseau, donc rien à détecter ni à monter.
- **Code supprimé**: `is_usb_tethering`, `USB_TETHER_MARKERS`, `STATUS_DISABLED`, `tethering_to_enable`, `AdapterListAge`, `tether_suppressed`, le bloc d'activation du monitor, l'événement `tethering-auto-enabled` et son toast.
- **Historique**: détection par marqueurs FR/EN (rndis, ndis, tethering, partage internet, android, iphone), état cible `Disabled`, anti-répétition par `HashSet`, activation via `spawn_blocking` au rythme du refresh adapters.
- **Leçon**: le branchement USB seul n'active pas le partage sur Android; il faut le cocher sur le téléphone (une fois). Ne pas réintroduire sans un moyen fiable côté téléphone.

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
