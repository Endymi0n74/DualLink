# Journal des versions — DualLink

Format inspiré de [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/), versions selon
[SemVer](https://semver.org/lang/fr/). Fenêtre Windows uniquement.

## [1.3.4] — 2026-10-29

Documentation et outillage. **Aucun changement de code** : binaire fonctionnellement identique à
la v1.3.3.

### Modifié

- `CHANGELOG.md` créé, historique consolidé depuis la v1.2.0.
- README FR et EN : badge de version, lien vers le journal des versions, section Téléchargement
  alignée sur les trois artefacts réellement publiés.
- `Agents.md` (ex `kumo agent.md`) : renommage et règle d'installation (tout sous `D:\codex`).

### Retiré

- Releases et tags v1.3.2 et v1.3.3, remplacés par celles de la v1.3.4.

## [1.3.3] — 2026-10-26

### Corrigé

- **Failover bloqué sur le secondaire avec une cible de ping en nom d'hôte** : `route.exe` n'accepte
  qu'une IPv4 numérique, la pose de la route `/32` échouait à chaque tick et était traduite en
  « primaire injoignable ». La cible est résolue en IPv4 à l'activation (`probe_target`) et
  l'activation est refusée si aucune adresse n'existe.
- **Toast du partage USB jamais affiché** : `showToast` manquait dans l'import de `app.js`, l'appel
  restait une globale inexistante (`ReferenceError`).

### Modifié

- `settings.json` écrit dans un fichier temporaire puis renommé : un arrêt pendant l'écriture ne
  laisse plus un JSON tronqué remplacé silencieusement par les défauts.

## [1.3.2] — 2026-10-20

### Ajouté

- **Partage USB du téléphone via ADB** : au branchement, la fonction USB du téléphone passe en
  `rndis` et la carte RNDIS apparaît seule. Réglages `adb_path` et `usb_tether_auto`, boutons
  « Tester ADB » et « Écran de partage ».

### Corrigé

- Processus sans délai maximal : un `Get-NetAdapter` bloqué pendant une ré-énumération USB gelait le
  monitor. Délais appliqués (PowerShell 20 s, ping/route 10 s, adb 15 s).

## [1.3.1] — 2026-10-20

### Retiré

- Montage automatique du partage USB (v1.3.0) : Android seul décide d'exposer une carte RNDIS, le PC
  ne peut pas déclencher le partage, la fonctionnalité n'avait pas de déclencheur fiable.

### Modifié

- Cible de connectivité par défaut `1.1.1.1` : `8.8.8.8` perdait jusqu'à 100 % des paquets sur la
  ligne testée et faisait clignoter le badge.
- Badge avec hystérésis (2 échecs pour perdre l'état, 1 succès pour le reprendre), « Vérification… »
  avant la première mesure.
- Sondes limitées aux cartes ayant une passerelle IPv4 valide.
- Liste des cartes rafraîchie à chaud quand le matériel change.
- Journal : rendu uniquement si le fichier a grossi.

## [1.3.0] — 2026-10-16

### Ajouté

- Montage automatique du partage USB du téléphone au branchement (retiré en v1.3.1).

## [1.2.3] — 2026-10-14

### Corrigé

- Métriques nominales rétablies lors d'une désactivation pendant une bascule, et à la sortie par le
  menu Quitter.

## [1.2.2] — 2026-10-11

### Ajouté

- Bannière persistante quand l'application tourne sans droits administrateur.

### Corrigé

- Back-end durci : nettoyage des routes `/32` orphelines au démarrage, indicateur `degraded` et
  `status_known` (une sonde de repli ne simule plus une carte joignable), `ensure_elevated` avant
  toute cmdlet, pin/unpin de route via `route.exe` natif.

## [1.2.1] — 2026-10-11

### Corrigé

- Manifeste UAC réellement embarqué via `tauri-build` : l'élévation demandée n'était jamais
  appliquée, l'exe démarrait en `asInvoker`.
- Chaque paramètre `windowsSettings` placé dans son espace de noms enregistré, sans quoi tout le
  manifeste est rejeté (erreur SxS 14001 au démarrage).

## [1.2.0] — 2026-10-11

### Modifié

- Versions alignées (`package.json`, `Cargo.toml`, `tauri.conf.json`, `app.manifest`) et métadonnées
  de version réparées.

---

Versions antérieures : voir les [releases GitHub](https://github.com/Endymi0n74/DualLink/releases).
