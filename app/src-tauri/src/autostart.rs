use std::fs;
use std::path::PathBuf;

fn autostart_path() -> PathBuf {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    base.join("autostart").join("repobell.desktop")
}

pub fn set_autostart(enabled: bool) -> std::io::Result<()> {
    let path = autostart_path();
    if enabled {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let current_exe = std::env::current_exe()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "repobell".to_string());

        let desktop_entry = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=RepoBell\n\
             Comment=RepoBell Git Watcher\n\
             Exec={}\n\
             Icon=repobell\n\
             Terminal=false\n\
             Categories=Development;\n",
            current_exe
        );
        fs::write(path, desktop_entry)?;
    } else if path.exists() {
        let _ = fs::remove_file(path);
    }
    Ok(())
}
