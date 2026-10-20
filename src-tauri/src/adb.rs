//! Pilotage du partage USB d'un téléphone Android via ADB.
//!
//! Android n'expose la carte RNDIS que si la fonction USB du téléphone passe
//! en `rndis`. Cette bascule est jouable depuis le PC:
//! `adb shell svc usb setFunctions rndis`. C'est le seul levier disponible
//! sans root, et il ne survit pas au débranchement: Android restaure sa
//! fonction configurée au rebranchement. La commande est donc rejouée à
//! chaque branchement.
//!
//! Le point d'accès Wi-Fi, lui, refuse la commande shell sur les builds
//! grand public: HyperOS lève une `SecurityException` sur
//! `cmd wifi set-wifi-ap-enabled`. On peut seulement ouvrir l'écran de
//! partage sur le téléphone.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Drapeau Windows: le processus enfant s'exécute sans fenêtre de console.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// État d'un appareil dans `adb devices`: autorisé et prêt.
pub const STATE_READY: &str = "device";

/// Délai maximal d'une commande adb (le serveur peut démarrer lentement).
const COMMAND_TIMEOUT_SECS: u64 = 15;

/// Marqueurs de la carte RNDIS dans la description Windows de l'adaptateur
/// (EN : « Remote NDIS based Internet Sharing Device », FR : « ... basé sur
/// NDIS distant »).
const RNDIS_MARKERS: &[&str] = &["remote ndis", "ndis distant", "rndis"];

/// Un téléphone visible par le serveur ADB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdbDevice {
    pub serial: String,
    pub state: String,
}

/// Issue d'une tentative d'activation du partage USB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// La carte RNDIS est déjà là: rien à commander.
    Present,
    /// Aucun téléphone autorisé n'est branché.
    NoDevice,
    /// La fonction USB du téléphone est passée en `rndis`.
    Armed(String),
    /// Échec: adb absent, appareil non autorisé, refus du téléphone.
    Failed(String),
}

/// Vrai si la description Windows désigne une carte RNDIS (partage USB).
pub fn is_rndis(description: &str) -> bool {
    let lowered = description.to_lowercase();
    RNDIS_MARKERS.iter().any(|marker| lowered.contains(marker))
}

/// Cherche `adb.exe`: chemin renseigné par l'utilisateur, puis PATH, puis
/// installations connues (winget, Android SDK).
pub fn locate(configured: &str) -> Option<PathBuf> {
    let custom = configured.trim();
    if !custom.is_empty() {
        let path = PathBuf::from(custom);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Ok(paths) = std::env::var("PATH") {
        for dir in paths.split(';') {
            if dir.trim().is_empty() {
                continue;
            }
            let candidate = Path::new(dir.trim()).join("adb.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    known_installs().into_iter().find(|path| path.is_file())
}

/// Installations d'Android Platform-Tools usuelles.
fn known_installs() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    let local = match std::env::var("LOCALAPPDATA") {
        Ok(dir) => dir,
        Err(_) => return candidates,
    };

    candidates.push(
        Path::new(&local)
            .join("Android")
            .join("Sdk")
            .join("platform-tools")
            .join("adb.exe"),
    );

    let packages = Path::new(&local)
        .join("Microsoft")
        .join("WinGet")
        .join("Packages");
    if let Ok(entries) = std::fs::read_dir(&packages) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("Google.PlatformTools") {
                candidates.push(entry.path().join("platform-tools").join("adb.exe"));
            }
        }
    }

    candidates
}

/// Extrait la liste des appareils d'une sortie `adb devices -l`.
pub fn parse_devices(stdout: &str) -> Vec<AdbDevice> {
    stdout
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('*') || line.starts_with("List of devices") {
                return None;
            }
            let mut parts = line.split_whitespace();
            Some(AdbDevice {
                serial: parts.next()?.to_string(),
                state: parts.next()?.to_string(),
            })
        })
        .collect()
}

/// Choisit l'appareil à commander: le premier autorisé.
pub fn plan(devices: &[AdbDevice]) -> Option<String> {
    devices
        .iter()
        .find(|device| device.state == STATE_READY)
        .map(|device| device.serial.clone())
}

/// Client ADB: garde le binaire et le téléphone visé.
pub struct Adb {
    path: PathBuf,
    serial: Option<String>,
}

impl Adb {
    /// Retrouve le binaire `adb.exe`.
    pub fn open(configured: &str) -> Result<Self, String> {
        let path = locate(configured).ok_or_else(|| {
            "adb introuvable (installez Android Platform-Tools ou renseignez le chemin)".to_string()
        })?;

        Ok(Self { path, serial: None })
    }

    /// État du partage USB, et activation si le téléphone n'expose pas encore
    /// la fonction `rndis`.
    ///
    /// La fonction USB demandée au téléphone fait foi: la liste des cartes
    /// Windows peut avoir jusqu'à 30 s de retard, le téléphone, lui, répond
    /// tout de suite.
    pub fn check(&mut self) -> Outcome {
        if self.serial.is_none() {
            let devices = match list_devices(&self.path) {
                Ok(devices) => devices,
                Err(e) => {
                    // Le serveur ADB peut être en train de redémarrer: un
                    // second essai suffit à le laisser repartir.
                    std::thread::sleep(Duration::from_millis(800));
                    match list_devices(&self.path) {
                        Ok(devices) => devices,
                        Err(_) => return Outcome::Failed(e),
                    }
                }
            };

            self.serial = plan(&devices);
            if self.serial.is_none() {
                return Outcome::NoDevice;
            }
        }

        let serial = self.serial.clone().unwrap_or_default();

        match usb_function(&self.path, &serial) {
            Some(function) if has_rndis(&function) => Outcome::Present,
            Some(_) => match set_rndis(&self.path, &serial) {
                Ok(()) => match usb_function(&self.path, &serial) {
                    // La bascule coupe la session ADB: sans réponse, la
                    // commande est présumée passée; avec une réponse, la
                    // fonction est vérifiée.
                    None => Outcome::Armed(serial),
                    Some(function) if has_rndis(&function) => Outcome::Armed(serial),
                    Some(function) => {
                        Outcome::Failed(format!("la fonction USB est restée '{function}'"))
                    }
                },
                Err(e) => {
                    self.serial = None;
                    Outcome::Failed(e)
                }
            },
            None => {
                // Le téléphone ne répond plus: il a été débranché.
                self.serial = None;
                Outcome::NoDevice
            }
        }
    }
}

/// Fonction USB demandée au téléphone ("rndis,adb", "adb", ...).
pub fn usb_function(adb: &Path, serial: &str) -> Option<String> {
    let output = exec(adb, &["-s", serial, "shell", "getprop", "sys.usb.config"]).ok()?;
    let value = output.stdout.trim().to_string();

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Vrai si la fonction USB inclut `rndis`.
pub fn has_rndis(function: &str) -> bool {
    function
        .split(',')
        .any(|part| part.trim().eq_ignore_ascii_case("rndis"))
}

/// Ouvre l'écran « Point d'accès et partage de connexion » sur le téléphone.
pub fn open_tethering_screen(configured_path: &str) -> Result<(), String> {
    let adb = locate(configured_path)
        .ok_or_else(|| "adb introuvable (installez Android Platform-Tools)".to_string())?;
    let devices = list_devices(&adb)?;
    let serial = plan(&devices).ok_or_else(|| "aucun téléphone autorisé".to_string())?;

    run(
        &adb,
        &[
            "-s",
            &serial,
            "shell",
            "am",
            "start",
            "-a",
            "android.settings.TETHER_SETTINGS",
        ],
    )?;
    Ok(())
}

/// Liste les appareils connus du serveur ADB.
pub fn list_devices(adb: &Path) -> Result<Vec<AdbDevice>, String> {
    let stdout = run(adb, &["devices", "-l"])?;
    Ok(parse_devices(&stdout))
}

/// Bascule la fonction USB du téléphone en `rndis`.
///
/// La bascule coupe la session ADB qui l'a demandée: la commande revient
/// souvent sans sortie. Ce n'est pas un échec, la fonction a été appliquée.
/// Seuls un refus explicite (SecurityException, Permission Denial) est une
/// erreur.
fn set_rndis(adb: &Path, serial: &str) -> Result<(), String> {
    let output = exec(
        adb,
        &["-s", serial, "shell", "svc", "usb", "setFunctions", "rndis"],
    )?;

    let reply = format!("{} {}", output.stdout, output.stderr);
    if reply.contains("Exception") || reply.contains("Permission Denial") {
        return Err(format!("refus du téléphone: {}", first_line(&reply)));
    }

    Ok(())
}

/// Exécute une commande adb sans fenêtre de console, avec un délai maximal.
fn exec(adb: &Path, args: &[&str]) -> Result<Output, String> {
    let mut command = Command::new(adb);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|e| format!("adb impossible à lancer: {e}"))?;

    let deadline = Instant::now() + Duration::from_secs(COMMAND_TIMEOUT_SECS);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    // Le processus est tué: on abandonne sa sortie plutôt que
                    // d'attendre un `wait` qui ne reviendrait pas.
                    return Err("adb: délai dépassé".to_string());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(format!("adb: {e}")),
        }
    }

    let output = child.wait_with_output().map_err(|e| format!("adb: {e}"))?;

    Ok(Output {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Sortie d'une commande adb.
struct Output {
    code: i32,
    stdout: String,
    stderr: String,
}

/// Exécute une commande adb et exige un code de retour nul.
fn run(adb: &Path, args: &[&str]) -> Result<String, String> {
    let output = exec(adb, args)?;

    if output.code != 0 {
        let reason = first_line(&output.stderr);
        let detail = if reason == "échec" {
            // Sortie vide: le code de retour et stdout disent ce qui s'est passé.
            format!("code {} — {}", output.code, first_line(&output.stdout))
        } else {
            reason
        };
        return Err(format!("adb: {detail}"));
    }

    Ok(output.stdout)
}

/// Première ligne non vide d'un message d'erreur.
fn first_line(message: &str) -> String {
    message
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("échec")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_la_liste_des_appareils() {
        let stdout = "List of devices attached\n\
                      QOGQ694DEE5PLBGI       device product:malachitep_eea model:24095PCADG\n\
                      \n\
                      * daemon started successfully\n\
                      ABC123               unauthorized\n";
        let devices = parse_devices(stdout);

        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].serial, "QOGQ694DEE5PLBGI");
        assert_eq!(devices[0].state, STATE_READY);
        assert_eq!(devices[1].state, "unauthorized");
    }

    #[test]
    fn choisit_un_appareil_autorise() {
        let devices = vec![
            AdbDevice {
                serial: "ABC".into(),
                state: "unauthorized".into(),
            },
            AdbDevice {
                serial: "XYZ".into(),
                state: STATE_READY.into(),
            },
        ];

        assert_eq!(plan(&devices), Some("XYZ".to_string()));
        assert_eq!(plan(&[]), None);
    }

    #[test]
    fn detecte_une_carte_rndis() {
        assert!(is_rndis("Remote NDIS based Internet Sharing Device #4"));
        assert!(is_rndis("rndis0"));
        assert!(is_rndis(
            "Périphérique de partage Internet basé sur NDIS distant"
        ));
        assert!(!is_rndis("Realtek Gaming 2.5GbE Family Controller"));
        assert!(!is_rndis("Hyper-V Virtual Ethernet Adapter"));
    }

    #[test]
    fn accepte_un_chemin_adb_valide() {
        let fake = std::env::temp_dir().join("dualink-adb-test.exe");
        std::fs::write(&fake, b"x").unwrap();

        assert_eq!(locate(fake.to_str().unwrap()), Some(fake.clone()));

        std::fs::remove_file(&fake).ok();
    }
}
