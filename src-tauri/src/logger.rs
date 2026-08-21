use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

static LOG_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

fn log_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(base).join("DualLink").join("logs")
}

pub fn init() {
    let dir = log_dir();
    let _ = fs::create_dir_all(&dir);

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let date = format_timestamp(now);
    let path = dir.join(format!("{}.log", &date));

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok();

    if let Some(ref mut f) = file {
        let _ = writeln!(f, "\n=== DualLink started at {} ===", format_datetime(now));
    }

    if let Ok(mut guard) = LOG_FILE.lock() {
        *guard = Some(path);
    }

    // Panic hook: log panics to file before aborting
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

        let location = info.location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".to_string());

        log_file(&format!("PANIC [{}] {} at {}", thread_name, msg, location));

        // Call the default hook (prints to stderr if console is visible)
        default_hook(info);
    }));
}

pub fn log_file(msg: &str) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let line = format!("[{}] {}\n", format_datetime(now), msg);

    if let Ok(guard) = LOG_FILE.lock() {
        if let Some(ref path) = *guard {
            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = file.write_all(line.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

// ── Helpers ──

fn format_datetime(secs: u64) -> String {
    let secs_in_day = secs % 86400;
    let h = secs_in_day / 3600;
    let m = (secs_in_day % 3600) / 60;
    let s = secs_in_day % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

fn format_timestamp(secs: u64) -> String {
    let days = secs / 86400;
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = 1 + (day_of_year / 30);
    let day = 1 + (day_of_year % 30);
    format!("{:04}-{:02}-{:02}", year, month.min(12), day.min(28))
}
