use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Watched folders.
    pub folders: Vec<PathBuf>,
    /// Repos the user chose "Stop watching" on.
    pub ignored: Vec<PathBuf>,
    /// Repos with notifications off.
    pub muted: Vec<PathBuf>,
    pub editor: Option<String>,
    pub check_minutes: u32,
    pub start_at_login: bool,
    /// True once first-run setup is done, so removing every folder never brings the wizard back.
    pub setup_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            folders: vec![],
            ignored: vec![],
            muted: vec![],
            editor: None,
            check_minutes: 5,
            start_at_login: true,
            setup_done: false,
        }
    }
}

/// Repo path -> last seen commit sha.
pub type Seen = BTreeMap<PathBuf, String>;

/// Repo path -> teammate change shas we already sent a notification for (or skipped silently).
pub type Notified = BTreeMap<PathBuf, Vec<String>>;

/// `$var` if set and absolute, else `$HOME/<fallback>`; then `repobell/<file>`.
fn data_path(var: &str, fallback: &str, file: &str) -> PathBuf {
    let base = std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback)
        });
    base.join("repobell").join(file)
}

pub fn settings_path() -> PathBuf {
    data_path("XDG_CONFIG_HOME", ".config", "settings.json")
}

pub fn seen_path() -> PathBuf {
    data_path("XDG_STATE_HOME", ".local/state", "seen.json")
}

pub fn notified_path() -> PathBuf {
    data_path("XDG_STATE_HOME", ".local/state", "notified.json")
}

/// Missing file gives defaults. A file that doesn't parse is moved to `settings.json.bad`
/// (replacing an older one) so the user's text is never overwritten.
pub fn load_settings(path: &Path) -> Settings {
    let Ok(bytes) = fs::read(path) else {
        return Settings::default();
    };
    serde_json::from_slice::<Settings>(&bytes)
        .map(|mut s| {
            // Files from before `setup_done` existed: watching folders means setup happened.
            s.setup_done |= !s.folders.is_empty();
            s
        })
        .unwrap_or_else(|_| {
            let mut bad = path.as_os_str().to_owned();
            bad.push(".bad");
            let _ = fs::rename(path, bad);
            Settings::default()
        })
}

pub fn load_seen(path: &Path) -> Seen {
    fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn load_notified(path: &Path) -> Notified {
    fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save_notified(path: &Path, n: &Notified) -> std::io::Result<()> {
    write_atomic(path, n)
}

pub fn save_settings(path: &Path, s: &Settings) -> std::io::Result<()> {
    write_atomic(path, s)
}

pub fn save_seen(path: &Path, s: &Seen) -> std::io::Result<()> {
    write_atomic(path, s)
}

/// Writes pretty JSON to a uniquely named temp file next to `path`, syncs it, then renames it
/// over `path`. The unique name keeps two writers (or two RepoBell copies) from sharing a temp file.
fn write_atomic<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let tmp = PathBuf::from(tmp);
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&serde_json::to_vec_pretty(value).map_err(std::io::Error::from)?)?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}
