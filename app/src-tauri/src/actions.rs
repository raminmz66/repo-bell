use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

use crate::autostart;
use crate::store::{self, Settings};
use crate::watcher::{AppStateView, SharedEngine};

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateOutcomeView {
    pub success: bool,
    pub downloads: u32,
    pub undo_head: Option<String>,
    pub conflict_files: Vec<String>,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestedFolder {
    pub path: PathBuf,
    pub display: String,
    pub repos: Vec<String>,
}

#[tauri::command]
pub async fn get_app_state(engine: State<'_, SharedEngine>) -> Result<AppStateView, String> {
    let state = engine.lock().await;
    Ok(state.to_view())
}

#[tauri::command]
pub async fn check_now(
    app: tauri::AppHandle,
    engine: State<'_, SharedEngine>,
) -> Result<AppStateView, String> {
    let (settings, seen) = {
        let state = engine.lock().await;
        (state.settings.clone(), state.seen.clone())
    };

    let (repos, offline) = crate::watcher::check_repositories(&settings, &seen);
    let now = crate::watcher::now_seconds();

    let mut state = engine.lock().await;
    state.repos = repos;
    state.offline = offline;
    state.last_checked = Some(now);

    // Update tray if tray exists
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = crate::watcher::update_tray(&app, &tray, &state);
    }

    Ok(state.to_view())
}

#[tauri::command]
pub async fn mark_all_seen(
    app: tauri::AppHandle,
    engine: State<'_, SharedEngine>,
) -> Result<AppStateView, String> {
    let mut state = engine.lock().await;
    let mut to_insert = Vec::new();
    for r in &state.repos {
        if let Ok(sha) = repobell_core::remote_main_sha(&r.path, &r.main_branch) {
            to_insert.push((r.path.clone(), sha));
        }
    }
    for (p, s) in to_insert {
        state.seen.insert(p, s);
    }
    let _ = store::save_seen(&store::seen_path(), &state.seen);

    // Clear feeds for all repos
    for r in &mut state.repos {
        r.feed = None;
    }

    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = crate::watcher::update_tray(&app, &tray, &state);
    }

    Ok(state.to_view())
}

#[tauri::command]
pub async fn save_settings(
    app: tauri::AppHandle,
    engine: State<'_, SharedEngine>,
    settings: Settings,
) -> Result<AppStateView, String> {
    let _ = store::save_settings(&store::settings_path(), &settings);
    let _ = autostart::set_autostart(settings.start_at_login);

    let local_repos = crate::watcher::discover_local_repositories(&settings);
    {
        let mut state = engine.lock().await;
        state.settings = settings.clone();
        state.repos = local_repos;
    }

    let bg_engine = engine.inner().clone();
    let bg_app = app.clone();
    let bg_settings = settings.clone();
    tauri::async_runtime::spawn(async move {
        let seen = {
            let state = bg_engine.lock().await;
            state.seen.clone()
        };
        let (repos, offline) = crate::watcher::check_repositories(&bg_settings, &seen);
        let mut state = bg_engine.lock().await;
        state.repos = repos;
        state.offline = offline;
        state.last_checked = Some(crate::watcher::now_seconds());
        if let Some(tray) = bg_app.tray_by_id("main-tray") {
            let _ = crate::watcher::update_tray(&bg_app, &tray, &state);
        }
        let _ = bg_app.emit("state-updated", state.to_view());
    });

    let state = engine.lock().await;
    if let Some(tray) = app.tray_by_id("main-tray") {
        let _ = crate::watcher::update_tray(&app, &tray, &state);
    }

    Ok(state.to_view())
}

#[tauri::command]
pub async fn update_repo(
    path: PathBuf,
    engine: State<'_, SharedEngine>,
) -> Result<UpdateOutcomeView, String> {
    let plan = repobell_core::update::plan(&path).map_err(|e| e.to_string())?;

    match repobell_core::update::update(&path, &plan) {
        Ok(repobell_core::Outcome::Updated { downloads, undo }) => {
            let mut state = engine.lock().await;
            if let Some(r) = state.repos.iter_mut().find(|r| r.path == path) {
                r.behind = 0;
                r.undo_head = Some(undo.before.clone());
            }
            Ok(UpdateOutcomeView {
                success: true,
                downloads,
                undo_head: Some(undo.before),
                conflict_files: Vec::new(),
                message: format!("Updated. {} change{} downloaded.", downloads, if downloads == 1 { "" } else { "s" }),
            })
        }
        Ok(repobell_core::Outcome::UpToDate) => Ok(UpdateOutcomeView {
            success: true,
            downloads: 0,
            undo_head: None,
            conflict_files: Vec::new(),
            message: "Already up to date.".to_string(),
        }),
        Ok(repobell_core::Outcome::Conflict { files }) => Ok(UpdateOutcomeView {
            success: false,
            downloads: 0,
            undo_head: None,
            conflict_files: files,
            message: "Update stopped. Nothing was changed.".to_string(),
        }),
        Ok(repobell_core::Outcome::PlanChanged(_)) => Err("Repository state changed. Please try again.".to_string()),
        Ok(repobell_core::Outcome::Blocked(b)) => Err(format!("Update blocked: {:?}", b)),
        Err(e) => Err(format!("Git error: {}", e)),
    }
}

#[tauri::command]
pub async fn undo_repo(
    path: PathBuf,
    undo_head: String,
    engine: State<'_, SharedEngine>,
) -> Result<bool, String> {
    let current_head = match repobell_core::status(&path) {
        Ok(_) => {
            // Get current HEAD sha
            let output = Command::new("git")
                .args(["-C", path.to_str().unwrap(), "rev-parse", "HEAD"])
                .output()
                .map_err(|e| e.to_string())?;
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        Err(e) => return Err(e.to_string()),
    };

    let undo_point = repobell_core::update::UndoPoint {
        before: undo_head,
        after: current_head,
    };

    match repobell_core::update::undo(&path, &undo_point) {
        Ok(repobell_core::UndoOutcome::Undone) => {
            let mut state = engine.lock().await;
            if let Some(r) = state.repos.iter_mut().find(|r| r.path == path) {
                r.undo_head = None;
                if let Ok(st) = repobell_core::status(&path) {
                    r.behind = st.behind;
                    r.ahead = st.ahead;
                    r.unsaved = st.unsaved;
                }
            }
            Ok(true)
        }
        Ok(repobell_core::UndoOutcome::NotPossible) => Ok(false),
        Err(e) => Err(format!("Undo failed: {}", e)),
    }
}

#[tauri::command]
pub async fn open_editor(
    file_path: PathBuf,
    engine: State<'_, SharedEngine>,
) -> Result<(), String> {
    let editor = {
        let state = engine.lock().await;
        state.settings.editor.clone()
    };

    let cmd_name = match editor.as_deref() {
        Some("VS Code") => "code",
        Some("Sublime Text") => "subl",
        Some("Text Editor") | Some("GNOME Text Editor") => "gnome-text-editor",
        _ => "xdg-open",
    };

    Command::new(cmd_name)
        .arg(&file_path)
        .spawn()
        .map_err(|e| format!("Failed to open editor: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn pick_folder() -> Result<Option<PathBuf>, String> {
    let output = Command::new("zenity")
        .args(["--file-selection", "--directory", "--title=Select Folder to Watch"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(PathBuf::from(s)))
            }
        }
        _ => Ok(None),
    }
}

#[tauri::command]
pub fn detect_editors() -> Vec<String> {
    let mut editors = Vec::new();
    // Check known commands
    let candidates = [
        ("code", "VS Code"),
        ("subl", "Sublime Text"),
        ("gnome-text-editor", "Text Editor"),
        ("gedit", "Text Editor"),
        ("idea", "IntelliJ IDEA"),
        ("pycharm", "PyCharm"),
        ("webstorm", "WebStorm"),
    ];

    for (cmd, name) in candidates {
        if Command::new("which").arg(cmd).output().map_or(false, |o| o.status.success()) {
            if !editors.contains(&name.to_string()) {
                editors.push(name.to_string());
            }
        }
    }

    editors.push("System default app".to_string());
    editors
}

#[tauri::command]
pub fn discover_suggested_folders() -> Vec<SuggestedFolder> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let candidates = [
        home.join("projects"),
        home.join("code"),
        home.join("work"),
        home.join("Documents"),
        home.join("design"),
    ];

    let mut results = Vec::new();
    for folder in candidates {
        if folder.exists() && folder.is_dir() {
            let repos = repobell_core::discover(&[folder.clone()], 3);
            if !repos.is_empty() {
                let repo_names: Vec<String> = repos
                    .iter()
                    .map(|p| p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned()))
                    .collect();

                let display = if folder.starts_with(&home) {
                    format!("~/{}", folder.strip_prefix(&home).unwrap().display())
                } else {
                    folder.display().to_string()
                };

                results.push(SuggestedFolder {
                    path: folder,
                    display,
                    repos: repo_names,
                });
            }
        }
    }
    results
}
