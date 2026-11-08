use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

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
        }
    }
}

/// Repo path -> last seen commit sha.
pub type Seen = BTreeMap<PathBuf, String>;

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

/// Missing file gives defaults. A file that doesn't parse is moved to `settings.json.bad`
/// (replacing an older one) so the user's text is never overwritten.
pub fn load_settings(path: &Path) -> Settings {
    let Ok(bytes) = fs::read(path) else {
        return Settings::default();
    };
    serde_json::from_slice(&bytes).unwrap_or_else(|_| {
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

pub fn save_settings(path: &Path, s: &Settings) -> std::io::Result<()> {
    write_atomic(path, s)
}

pub fn save_seen(path: &Path, s: &Seen) -> std::io::Result<()> {
    write_atomic(path, s)
}

/// Writes pretty JSON to `<path>.tmp`, syncs it, then renames it over `path`.
fn write_atomic<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let mut f = fs::File::create(&tmp)?;
    f.write_all(&serde_json::to_vec_pretty(value).map_err(std::io::Error::from)?)?;
    f.sync_all()?;
    fs::rename(&tmp, path)
}
