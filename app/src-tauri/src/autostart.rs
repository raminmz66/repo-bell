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

/// The `Exec=` value for `program`, quoted as the Desktop Entry spec asks: double quotes around
/// the path, a backslash before `"`, `` ` `` and `$`, four backslashes for a literal backslash
/// (the file's own string escaping eats one level), and `%` written as `%%`.
pub fn exec_line(program: &str) -> String {
    let mut out = String::from("\"");
    for c in program.chars() {
        match c {
            '"' | '`' | '$' => {
                out.push('\\');
                out.push(c);
            }
            '\\' => out.push_str("\\\\\\\\"),
            '%' => out.push_str("%%"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// An AppImage runs from a temporary mount, so `$APPIMAGE` (the real file) is what must be
/// started at login. Otherwise the running binary is used.
fn program_path() -> String {
    std::env::var("APPIMAGE")
        .ok()
        .filter(|p| !p.is_empty())
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.to_str().map(String::from))
        })
        .unwrap_or_else(|| "repobell".to_string())
}

pub fn desktop_entry(program: &str) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=RepoBell\n\
         Comment=RepoBell Git Watcher\n\
         Exec={}\n\
         Icon=repobell\n\
         Terminal=false\n\
         Categories=Development;\n",
        exec_line(program)
    )
}

pub fn set_autostart(enabled: bool) -> std::io::Result<()> {
    let path = autostart_path();
    if enabled {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, desktop_entry(&program_path()))?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_path_is_quoted() {
        assert_eq!(exec_line("/usr/bin/repobell"), "\"/usr/bin/repobell\"");
    }

    #[test]
    fn spaces_percent_and_specials() {
        assert_eq!(exec_line("/opt/My App/100%/r"), "\"/opt/My App/100%%/r\"");
        assert_eq!(exec_line("/a\"b`c$d"), "\"/a\\\"b\\`c\\$d\"");
        assert_eq!(exec_line("/a\\b"), "\"/a\\\\\\\\b\"");
    }

    #[test]
    fn entry_has_the_quoted_exec() {
        let e = desktop_entry("/tmp/Repo Bell.AppImage");
        assert!(e.contains("\nExec=\"/tmp/Repo Bell.AppImage\"\n"));
        assert!(e.starts_with("[Desktop Entry]\n"));
    }
}
