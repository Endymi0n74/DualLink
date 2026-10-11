use chrono::{Local, NaiveDate};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// Sérialise les écritures concurrentes dans les fichiers de log.
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// Format des noms de fichiers de log: `AAAA-MM-JJ.log`.
const LOG_FILE_DATE_FORMAT: &str = "%Y-%m-%d";

fn log_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("DualLink").join("logs")
}

/// Chemin du log du jour.
///
/// Recalculé à chaque appel: après minuit les écritures basculent
/// automatiquement dans le fichier du nouveau jour.
fn log_file_path() -> PathBuf {
    log_dir().join(format!("{}.log", Local::now().format(LOG_FILE_DATE_FORMAT)))
}

/// Vérifie qu'une date fournie par le frontend est bien un `AAAA-MM-JJ`.
///
/// La longueur est contrôlée avant chrono, qui accepte les formes padées
/// (`2026-8-22`) et produirait un nom de fichier hors convention. Bloque
/// aussi la traversée de chemin (`../../`) sur `read_log_file`.
fn is_valid_log_date(date: &str) -> bool {
    date.len() == 10 && NaiveDate::parse_from_str(date, LOG_FILE_DATE_FORMAT).is_ok()
}

pub fn init() {
    let dir = log_dir();
    if let Err(e) = fs::create_dir_all(&dir) {
        eprintln!("DualLink: création du dossier de logs impossible: {e}");
    }

    let start = format!(
        "\n=== DualLink started at {} ===",
        Local::now().format("%Y-%m-%d %H:%M:%S")
    );
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path())
    {
        let _ = writeln!(f, "{start}");
    }

    // Hook de panique: écrit dans le log avant d'appeler le hook par défaut.
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current();
        let thread_name = thread.name().unwrap_or("unnamed");

        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Box<dyn Any>".to_string()
        };

        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());

        // `try_lock`, jamais `lock`: si la panique survient pendant une
        // écriture de `log_file` (verrou détenu), un `lock` ici bloquerait
        // définitivement le thread en panique. Le verrou est pris quand il
        // est libre (écriture serialisée), et l'écriture a lieu dans tous
        // les cas, au risque d'un mélange de lignes.
        let line = format!(
            "[{}] PANIC [{}] {} at {}\n",
            Local::now().format("%H:%M:%S"),
            thread_name,
            msg,
            location
        );
        let _guard = WRITE_LOCK.try_lock().ok();
        append_line(&line);

        default_hook(info);
    }));
}

/// Ouvre le log du jour en écriture et y ajoute `line` (sans verrou).
fn append_line(line: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file_path())
    {
        let _ = file.write_all(line.as_bytes());
        let _ = file.flush();
    }
}

/// Écrit une ligne horodatée en heure locale dans le log du jour.
pub fn log_file(msg: &str) {
    let line = format!("[{}] {}\n", Local::now().format("%H:%M:%S"), msg);

    let _guard = WRITE_LOCK.lock();
    append_line(&line);
}

/// Read today's log file content
pub fn read_today_log() -> String {
    fs::read_to_string(log_file_path()).unwrap_or_default()
}

/// List all available log files
pub fn list_log_files() -> Vec<String> {
    let dir = log_dir();
    let mut files: Vec<String> = fs::read_dir(&dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "log")
                        .unwrap_or(false)
                })
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// Read a specific log file by date string (e.g. "2026-08-22")
///
/// La date est validée avant tout usage: une entrée invalide
/// (`../../secret`, `2026/08/22`, …) retourne une chaîne vide.
pub fn read_log_file(date: &str) -> String {
    if !is_valid_log_date(date) {
        return String::new();
    }
    let path = log_dir().join(format!("{date}.log"));
    fs::read_to_string(&path).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_date_valide_accepte_aaaa_mm_jj() {
        assert!(is_valid_log_date("2026-08-22"));
        assert!(is_valid_log_date("2024-02-29"));
    }

    #[test]
    fn log_date_valide_refuse_traversee_et_format_incorrect() {
        for date in [
            "../../x",
            "..\\..\\x",
            "2026/08/22",
            "2026-8-22.log",
            "2026-8-22",
            "20260822",
            "",
            "aujourd'hui",
            "2026-13-01",
        ] {
            assert!(!is_valid_log_date(date), "date acceptée à tort: {date}");
        }
    }

    #[test]
    fn read_log_file_traversee_vide() {
        assert_eq!(read_log_file("../../x"), "");
        assert_eq!(
            read_log_file("..\\..\\Windows\\System32\\drivers\\etc\\hosts"),
            ""
        );
    }

    #[test]
    fn read_log_file_date_valide_fichier_absent_vide() {
        assert_eq!(read_log_file("2000-01-01"), "");
    }
}
