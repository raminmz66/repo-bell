//! Finds installed editors from `.desktop` files and opens files through them (no shell).

use std::path::{Path, PathBuf};
use std::process::Command;

pub const SYSTEM_DEFAULT: &str = "System default app";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopEntry {
    /// File name, such as `org.gnome.gedit.desktop`.
    pub id: String,
    pub name: String,
    pub exec: String,
}

/// The `[Desktop Entry]` group of a `.desktop` file. `None` for entries that are not
/// applications, are hidden from menus (`NoDisplay`, `Hidden`) or have no name or command.
pub fn parse_desktop(id: &str, text: &str) -> Option<DesktopEntry> {
    let (mut name, mut exec, mut kind) = (None, None, None);
    let (mut hidden, mut in_group) = (false, false);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_group = line == "[Desktop Entry]";
        } else if in_group {
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            match k.trim() {
                "Name" => name = Some(v.trim().to_string()),
                "Exec" => exec = Some(v.trim().to_string()),
                "Type" => kind = Some(v.trim().to_string()),
                "NoDisplay" | "Hidden" if v.trim() == "true" => hidden = true,
                _ => {}
            }
        }
    }
    if hidden || kind.as_deref() != Some("Application") {
        return None;
    }
    Some(DesktopEntry {
        id: id.to_string(),
        name: name?,
        exec: exec.filter(|e| !e.is_empty())?,
    })
}

pub fn app_dirs() -> Vec<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    vec![
        PathBuf::from("/usr/share/applications"),
        home.join(".local/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        home.join(".local/share/flatpak/exports/share/applications"),
    ]
}

pub fn load_entries(dirs: &[PathBuf]) -> Vec<DesktopEntry> {
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        let mut files: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        files.sort();
        for p in files {
            let Some(id) = p.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !id.ends_with(".desktop") {
                continue;
            }
            if let Some(e) = std::fs::read_to_string(&p)
                .ok()
                .and_then(|t| parse_desktop(id, &t))
            {
                out.push(e);
            }
        }
    }
    out
}

const JETBRAINS: &[&str] = &[
    "IntelliJ IDEA",
    "PyCharm",
    "WebStorm",
    "PhpStorm",
    "GoLand",
    "RubyMine",
    "CLion",
    "Rider",
    "DataGrip",
    "RustRover",
    "DataSpell",
    "Aqua",
];

/// The display name RepoBell uses for a known editor, or `None` for any other app.
/// This is also the string saved in settings, so it must stay stable across upgrades.
pub fn known_name(e: &DesktopEntry) -> Option<String> {
    let id = e.id.trim_end_matches(".desktop").to_lowercase();
    let id = id.as_str();
    if matches!(
        id,
        "code"
            | "code-oss"
            | "code_code"
            | "com.visualstudio.code"
            | "visual-studio-code"
            | "vscodium"
            | "codium"
            | "com.vscodium.codium"
    ) {
        return Some("VS Code".into());
    }
    if e.name.starts_with("Sublime Text")
        || id.contains("sublime_text")
        || id.contains("sublime-text")
    {
        return Some("Sublime Text".into());
    }
    if matches!(id, "org.gnome.texteditor" | "org.gnome.gedit" | "gedit") {
        return Some("Text Editor".into());
    }
    JETBRAINS
        .iter()
        .find(|p| e.name.starts_with(*p))
        .map(|p| p.to_string())
}

/// Installed known editors (one entry per display name; earlier directories win; the GNOME
/// Text Editor is preferred over gedit) followed by the system default.
pub fn detect(entries: &[DesktopEntry]) -> Vec<(String, Option<DesktopEntry>)> {
    let mut found: Vec<(String, Option<DesktopEntry>)> = Vec::new();
    for e in entries {
        let Some(n) = known_name(e) else { continue };
        match found.iter_mut().find(|(f, _)| *f == n) {
            None => found.push((n, Some(e.clone()))),
            Some((_, slot)) if e.id == "org.gnome.TextEditor.desktop" => *slot = Some(e.clone()),
            Some(_) => {}
        }
    }
    found.push((SYSTEM_DEFAULT.to_string(), None));
    found
}

/// Settings written by older versions used command names; map them to display names.
fn normalize(setting: &str) -> &str {
    match setting {
        "code" => "VS Code",
        "subl" => "Sublime Text",
        "gnome-text-editor" | "gedit" | "GNOME Text Editor" => "Text Editor",
        other => other,
    }
}

/// Splits an `Exec` line into arguments following the Desktop Entry quoting rules, replaces
/// `%f %F %u %U` with `file`, `%%` with `%`, and drops the other field codes. When the line
/// has no file code the file is appended. Nothing here ever reaches a shell.
pub fn exec_args(exec: &str, file: &str) -> Option<Vec<String>> {
    let mut args: Vec<String> = Vec::new();
    let mut cur = String::new();
    let (mut in_arg, mut quoted, mut used_file) = (false, false, false);
    let mut chars = exec.chars();
    while let Some(c) = chars.next() {
        if matches!(c, ' ' | '\t') && !quoted {
            if in_arg {
                args.push(std::mem::take(&mut cur));
                in_arg = false;
            }
            continue;
        }
        in_arg = true;
        match c {
            '"' => quoted = !quoted,
            '\\' if quoted => cur.extend(chars.next()),
            '%' => match chars.next() {
                Some('%') => cur.push('%'),
                Some('f' | 'F' | 'u' | 'U') => {
                    cur.push_str(file);
                    used_file = true;
                }
                _ => {}
            },
            c => cur.push(c),
        }
    }
    if quoted {
        return None;
    }
    if in_arg {
        args.push(cur);
    }
    // A bare field code that expanded to nothing (such as `%i`) leaves no empty argument.
    args.retain(|a| !a.is_empty());
    if !used_file {
        args.push(file.to_string());
    }
    (!args.is_empty()).then_some(args)
}

fn spawn(args: &[String]) -> Result<(), String> {
    let (cmd, rest) = args.split_first().ok_or("No program to run")?;
    let mut child = Command::new(cmd)
        .args(rest)
        .spawn()
        .map_err(|_| "Couldn't open the file in your editor.".to_string())?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Opens `file` with the editor saved in settings, or the system default.
pub fn open(editor: Option<&str>, file: &Path) -> Result<(), String> {
    let file_str = file.to_str().ok_or("This file name can't be opened.")?;
    let wanted = editor.map(normalize).filter(|e| *e != SYSTEM_DEFAULT);
    let entry = wanted.and_then(|w| {
        detect(&load_entries(&app_dirs()))
            .into_iter()
            .find(|(n, _)| n == w)
            .and_then(|(_, e)| e)
    });
    let args = match entry {
        Some(e) => exec_args(&e.exec, file_str),
        None => None,
    }
    .unwrap_or_else(|| vec!["xdg-open".to_string(), file_str.to_string()]);
    spawn(&args)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GEDIT: &str = "[Desktop Entry]\nName=Text Editor\nName[fr]=Éditeur\nExec=gedit %U\nType=Application\nCategories=GNOME;TextEditor;\n\n[Desktop Action new-window]\nName=New Window\nExec=gedit --new-window\n";

    #[test]
    fn parses_main_group_only() {
        let e = parse_desktop("org.gnome.gedit.desktop", GEDIT).unwrap();
        assert_eq!(e.name, "Text Editor");
        assert_eq!(e.exec, "gedit %U");
        assert_eq!(known_name(&e).as_deref(), Some("Text Editor"));
    }

    #[test]
    fn skips_hidden_and_non_apps() {
        let hidden =
            "[Desktop Entry]\nType=Application\nName=JetBrains\nNoDisplay=true\nExec=x %u\n";
        assert!(parse_desktop("jetbrainsd.desktop", hidden).is_none());
        assert!(
            parse_desktop("a.desktop", "[Desktop Entry]\nType=Link\nName=A\nExec=a\n").is_none()
        );
        assert!(
            parse_desktop("a.desktop", "[Desktop Entry]\nType=Application\nName=A\n").is_none()
        );
    }

    #[test]
    fn matches_known_editors() {
        let e = |id: &str, name: &str| DesktopEntry {
            id: id.into(),
            name: name.into(),
            exec: "x".into(),
        };
        assert_eq!(
            known_name(&e("code.desktop", "Visual Studio Code")).as_deref(),
            Some("VS Code")
        );
        assert_eq!(
            known_name(&e("phpstorm_phpstorm.desktop", "PhpStorm")).as_deref(),
            Some("PhpStorm")
        );
        assert_eq!(
            known_name(&e("jetbrains-idea.desktop", "IntelliJ IDEA Ultimate")).as_deref(),
            Some("IntelliJ IDEA")
        );
        assert_eq!(
            known_name(&e("sublime_text.desktop", "Sublime Text")).as_deref(),
            Some("Sublime Text")
        );
        assert_eq!(
            known_name(&e("sublime_merge.desktop", "Sublime Merge")),
            None
        );
        assert_eq!(
            known_name(&e("sublime-merge_smerge.desktop", "Sublime Merge")),
            None
        );
        assert_eq!(known_name(&e("firefox.desktop", "Firefox")), None);
        assert_eq!(known_name(&e("jetbrainsd.desktop", "JetBrains")), None);
    }

    #[test]
    fn detect_prefers_gnome_text_editor_and_ends_with_default() {
        let e = |id: &str, name: &str| DesktopEntry {
            id: id.into(),
            name: name.into(),
            exec: "x".into(),
        };
        let found = detect(&[
            e("org.gnome.gedit.desktop", "Text Editor"),
            e("org.gnome.TextEditor.desktop", "Text Editor"),
        ]);
        assert_eq!(found.len(), 2);
        assert_eq!(
            found[0].1.as_ref().unwrap().id,
            "org.gnome.TextEditor.desktop"
        );
        assert_eq!(found[1], (SYSTEM_DEFAULT.to_string(), None));
    }

    #[test]
    fn exec_field_codes_and_quoting() {
        let v = |s: &str| exec_args(s, "/a b/f.txt").unwrap();
        assert_eq!(v("gedit %U"), ["gedit", "/a b/f.txt"]);
        assert_eq!(
            v("/snap/bin/phpstorm %f"),
            ["/snap/bin/phpstorm", "/a b/f.txt"]
        );
        assert_eq!(
            v("code --unity-launch %F"),
            ["code", "--unity-launch", "/a b/f.txt"]
        );
        assert_eq!(v("edit --file=%f"), ["edit", "--file=/a b/f.txt"]);
        assert_eq!(v("edit"), ["edit", "/a b/f.txt"]);
        assert_eq!(v("edit %i %c %k %U"), ["edit", "/a b/f.txt"]);
        assert_eq!(
            v(r#""/opt/My App/run" --x 100%% %f"#),
            ["/opt/My App/run", "--x", "100%", "/a b/f.txt"]
        );
        assert_eq!(
            v(r#"run "a \"b\" \\ c" %f"#),
            ["run", r#"a "b" \ c"#, "/a b/f.txt"]
        );
        // The file name is one argument even with shell characters in it.
        assert_eq!(
            exec_args("edit %f", "; rm -rf ~").unwrap(),
            ["edit", "; rm -rf ~"]
        );
        assert!(exec_args("edit \"unterminated", "f").is_none());
    }

    #[test]
    fn old_settings_values_still_resolve() {
        assert_eq!(normalize("GNOME Text Editor"), "Text Editor");
        assert_eq!(normalize("code"), "VS Code");
        assert_eq!(normalize("PhpStorm"), "PhpStorm");
    }

    /// Reads the real .desktop files on this machine. Never fails; run with --nocapture to see.
    #[test]
    fn detect_on_this_machine() {
        let found = detect(&load_entries(&app_dirs()));
        println!("detected editors: {found:#?}");
        assert_eq!(found.last().unwrap().0, SYSTEM_DEFAULT);
    }
}
