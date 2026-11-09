use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use repobell_lib::store::{
    load_seen, load_settings, save_seen, save_settings, seen_path, settings_path, Seen, Settings,
};
use tempfile::TempDir;

fn sample_settings() -> Settings {
    Settings {
        folders: vec![PathBuf::from("/home/u/code")],
        ignored: vec![PathBuf::from("/home/u/code/old")],
        muted: vec![PathBuf::from("/home/u/code/noisy")],
        editor: Some("code".to_string()),
        check_minutes: 15,
        start_at_login: false,
        setup_done: true,
    }
}

#[test]
fn missing_settings_file_gives_default() {
    let dir = TempDir::new().unwrap();
    let s = load_settings(&dir.path().join("settings.json"));
    assert_eq!(s, Settings::default());

    let d = Settings::default();
    assert_eq!(d.check_minutes, 5);
    assert!(d.start_at_login);
    assert!(d.folders.is_empty());
    assert!(d.ignored.is_empty());
    assert!(d.muted.is_empty());
    assert_eq!(d.editor, None);
}

#[test]
fn settings_round_trip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    let s = sample_settings();
    save_settings(&path, &s).unwrap();
    assert_eq!(load_settings(&path), s);
}

#[test]
fn save_creates_missing_parent_dirs() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("tmp/a/b/settings.json");
    save_settings(&path, &Settings::default()).unwrap();
    assert!(path.is_file());
}

#[test]
fn save_leaves_no_tmp_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    save_settings(&path, &Settings::default()).unwrap();
    let names: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(names, vec!["settings.json".to_string()]);
}

#[test]
fn partial_settings_fill_defaults() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"check_minutes": 30}"#).unwrap();
    let s = load_settings(&path);
    assert_eq!(
        s,
        Settings {
            check_minutes: 30,
            ..Settings::default()
        }
    );
}

#[test]
fn broken_settings_move_to_bad_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    let bad = dir.path().join("settings.json.bad");

    fs::write(&path, "{not json").unwrap();
    assert_eq!(load_settings(&path), Settings::default());
    assert_eq!(fs::read_to_string(&bad).unwrap(), "{not json");
    assert!(!path.exists());

    // A second broken file replaces the older .bad.
    fs::write(&path, "{also broken").unwrap();
    assert_eq!(load_settings(&path), Settings::default());
    assert_eq!(fs::read_to_string(&bad).unwrap(), "{also broken");

    // Bytes that aren't UTF-8 count as broken too, so a later save can't overwrite them.
    fs::write(&path, b"{\"folders\": [\"\xff\"]}").unwrap();
    assert_eq!(load_settings(&path), Settings::default());
    assert!(!path.exists());
    assert_eq!(fs::read(&bad).unwrap(), b"{\"folders\": [\"\xff\"]}");
}

#[test]
fn save_overwrites_existing_settings() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    let a = sample_settings();
    let mut b = sample_settings();
    b.check_minutes = 60;
    save_settings(&path, &a).unwrap();
    save_settings(&path, &b).unwrap();
    assert_eq!(load_settings(&path), b);
}

#[test]
fn seen_missing_or_broken_is_empty() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("seen.json");
    assert!(load_seen(&path).is_empty());

    fs::write(&path, "{not json").unwrap();
    assert!(load_seen(&path).is_empty());
}

#[test]
fn seen_round_trip_with_non_ascii_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("seen.json");
    let mut seen: Seen = BTreeMap::new();
    seen.insert(PathBuf::from("/home/u/repo-one"), "abc123".to_string());
    seen.insert(PathBuf::from("/home/u/پروژه"), "def456".to_string());
    save_seen(&path, &seen).unwrap();
    assert_eq!(load_seen(&path), seen);
}

#[test]
fn env_paths_absolute_and_relative() {
    let home = TempDir::new().unwrap();
    let config = TempDir::new().unwrap();
    let state = TempDir::new().unwrap();

    std::env::set_var("HOME", home.path());

    // Absolute XDG vars win.
    std::env::set_var("XDG_CONFIG_HOME", config.path());
    std::env::set_var("XDG_STATE_HOME", state.path());
    assert_eq!(
        settings_path(),
        config.path().join("repobell").join("settings.json")
    );
    assert_eq!(seen_path(), state.path().join("repobell").join("seen.json"));

    // Relative XDG vars are ignored and HOME fallbacks are used.
    std::env::set_var("XDG_CONFIG_HOME", "rel");
    std::env::set_var("XDG_STATE_HOME", "rel");
    assert_eq!(
        settings_path(),
        home.path().join(".config/repobell/settings.json")
    );
    assert_eq!(
        seen_path(),
        home.path().join(".local/state/repobell/seen.json")
    );
}

#[test]
fn notified_round_trip_and_unique_temp_names() {
    use repobell_lib::store::{load_notified, save_notified, Notified};
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("notified.json");
    assert!(load_notified(&path).is_empty());
    let mut n = Notified::new();
    n.insert(PathBuf::from("/r/a"), vec!["1".into(), "2".into()]);
    save_notified(&path, &n).unwrap();
    save_notified(&path, &n).unwrap();
    assert_eq!(load_notified(&path), n);
    let files: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(files.len(), 1, "no temp file left behind");
    fs::write(&path, "{broken").unwrap();
    assert!(load_notified(&path).is_empty());
}

#[test]
fn old_settings_with_folders_count_as_setup_done() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("settings.json");
    fs::write(&path, r#"{"folders": ["/a"]}"#).unwrap();
    assert!(load_settings(&path).setup_done);
    fs::write(&path, r#"{"check_minutes": 30}"#).unwrap();
    assert!(!load_settings(&path).setup_done);
}
