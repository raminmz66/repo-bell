use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::async_runtime::Mutex;
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIcon;
use tauri::AppHandle;

use crate::store::{self, Seen, Settings};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeView {
    pub author: String,
    pub title: String,
    pub sha: String,
    pub timestamp: i64,
    pub mine: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSummary {
    pub total_changes: usize,
    pub author_summary: String,
    pub newest_time: Option<i64>,
    pub changes: Vec<ChangeView>,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoCardView {
    pub path: PathBuf,
    pub name: String,
    pub folder: PathBuf,
    pub main_branch: String,
    pub error: Option<String>,
    pub behind: u32,
    pub ahead: u32,
    pub unsaved: bool,
    pub muted: bool,
    pub ignored: bool,
    pub feed: Option<FeedSummary>,
    pub undo_head: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateView {
    pub repos: Vec<RepoCardView>,
    pub settings: Settings,
    pub last_checked: Option<u64>,
    pub offline: bool,
    pub first_run: bool,
}

pub struct EngineState {
    pub settings: Settings,
    pub seen: Seen,
    pub repos: Vec<RepoCardView>,
    pub last_checked: Option<u64>,
    pub offline: bool,
}

pub type SharedEngine = Arc<Mutex<EngineState>>;

impl EngineState {
    pub fn new() -> Self {
        let settings = store::load_settings(&store::settings_path());
        let seen = store::load_seen(&store::seen_path());
        let repos = discover_local_repositories(&settings);
        Self {
            settings,
            seen,
            repos,
            last_checked: None,
            offline: false,
        }
    }

    pub fn to_view(&self) -> AppStateView {
        let first_run = self.settings.folders.is_empty();
        AppStateView {
            repos: self.repos.clone(),
            settings: self.settings.clone(),
            last_checked: self.last_checked,
            offline: self.offline,
            first_run,
        }
    }
}

pub fn apply_settings_to_repos(repos: &mut [RepoCardView], settings: &Settings) {
    for r in repos.iter_mut() {
        r.ignored = settings.ignored.contains(&r.path);
        r.muted = settings.muted.contains(&r.path);
    }
}


pub fn discover_local_repositories(settings: &Settings) -> Vec<RepoCardView> {
    if settings.folders.is_empty() {
        return Vec::new();
    }
    let repo_paths = repobell_core::discover(&settings.folders, 4);
    let mut views = Vec::new();

    for path in repo_paths {
        let ignored = settings.ignored.contains(&path);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        let folder = settings
            .folders
            .iter()
            .find(|f| path.starts_with(f))
            .cloned()
            .unwrap_or_else(|| {
                path.parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| path.clone())
            });

        let muted = settings.muted.contains(&path);
        let mut error = None;
        let mut behind = 0;
        let mut ahead = 0;
        let mut unsaved = false;
        let mut main_branch = "main".to_string();

        if !ignored {
            match repobell_core::status(&path) {
                Ok(s) => {
                    behind = s.behind;
                    ahead = s.ahead;
                    unsaved = s.unsaved;
                    if let Some(mb) = s.main_branch {
                        main_branch = mb;
                    }
                }
                Err(e) => {
                    let problem = repobell_core::classify(&e, &path);
                    error = Some(problem.message().to_string());
                }
            }
        }

        views.push(RepoCardView {
            path,
            name,
            folder,
            main_branch,
            error,
            behind,
            ahead,
            unsaved,
            muted,
            ignored,
            feed: None,
            undo_head: None,
        });
    }
    views
}

pub fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn check_repositories(settings: &Settings, seen: &Seen) -> (Vec<RepoCardView>, bool) {
    if settings.folders.is_empty() {
        return (Vec::new(), false);
    }

    let repo_paths = repobell_core::discover(&settings.folders, 4);
    let mut views = Vec::new();
    let mut total_repos = 0usize;
    let mut offline_errors = 0usize;

    for path in repo_paths {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());

        let folder = settings
            .folders
            .iter()
            .find(|f| path.starts_with(f))
            .cloned()
            .unwrap_or_else(|| {
                path.parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| path.clone())
            });

        let muted = settings.muted.contains(&path);
        let ignored = settings.ignored.contains(&path);

        if ignored {
            views.push(RepoCardView {
                path,
                name,
                folder,
                main_branch: "main".to_string(),
                error: None,
                behind: 0,
                ahead: 0,
                unsaved: false,
                muted,
                ignored: true,
                feed: None,
                undo_head: None,
            });
            continue;
        }

        total_repos += 1;
        let mut error = None;
        let mut behind = 0;
        let mut ahead = 0;
        let mut unsaved = false;
        let mut main_branch = "main".to_string();

        // Check local status first to determine the remote main branch
        if let Ok(st) = repobell_core::status(&path) {
            behind = st.behind;
            ahead = st.ahead;
            unsaved = st.unsaved;
            if let Some(mb) = st.main_branch {
                main_branch = mb;
            }
        }

        // 1. Fetch remote main
        if let Err(e) = repobell_core::fetch_main(&path, &main_branch) {
            let problem = repobell_core::classify(&e, &path);
            if matches!(problem, repobell_core::Problem::Offline) {
                offline_errors += 1;
            }
            error = Some(problem.message().to_string());
        }

        // Re-read status after fetch for updated behind count
        match repobell_core::status(&path) {
            Ok(s) => {
                behind = s.behind;
                ahead = s.ahead;
                unsaved = s.unsaved;
                if let Some(mb) = s.main_branch {
                    main_branch = mb;
                }
            }
            Err(e) => {
                if error.is_none() {
                    let problem = repobell_core::classify(&e, &path);
                    error = Some(problem.message().to_string());
                }
            }
        }

        // 2. Feed
        let last_seen_sha = seen.get(&path).map(|s| s.as_str());
        let feed = match repobell_core::feed(&path, &main_branch, last_seen_sha, 5) {
            Ok(f) if f.total > 0 => {
                let author_summary = if let Some(first) = f.changes.first() {
                    let author_name = if first.mine {
                        "You".to_string()
                    } else {
                        first.author_name.clone()
                    };
                    format!(
                        "{} added {} change{} to the main version",
                        author_name,
                        f.total,
                        if f.total == 1 { "" } else { "s" }
                    )
                } else {
                    format!(
                        "{} change{} on the main version",
                        f.total,
                        if f.total == 1 { "" } else { "s" }
                    )
                };

                let newest_time = f.changes.first().map(|c| c.time);
                let changes = f
                    .changes
                    .into_iter()
                    .map(|c| ChangeView {
                        author: c.author_name,
                        title: c.title,
                        sha: c.sha,
                        timestamp: c.time,
                        mine: c.mine,
                    })
                    .collect();

                Some(FeedSummary {
                    total_changes: f.total,
                    author_summary,
                    newest_time,
                    changes,
                    files_changed: f.files_changed,
                })
            }
            _ => None,
        };

        views.push(RepoCardView {
            path,
            name,
            folder,
            main_branch,
            error,
            behind,
            ahead,
            unsaved,
            muted,
            ignored: false,
            feed,
            undo_head: None,
        });
    }

    let is_all_offline = total_repos > 0 && offline_errors == total_repos;
    (views, is_all_offline)
}

pub fn update_tray(app: &AppHandle, tray: &TrayIcon, state: &EngineState) -> tauri::Result<()> {
    let has_news_count = state
        .repos
        .iter()
        .filter(|r| !r.ignored && !r.muted && (r.behind > 0 || r.feed.as_ref().map_or(0, |f| f.total_changes) > 0))
        .count();

    // 1. Choose icon
    let icon_bytes = if state.offline {
        include_bytes!("../icons/tray/bell-off.png").as_slice()
    } else if has_news_count > 0 {
        include_bytes!("../icons/tray/bell-dot.png").as_slice()
    } else {
        include_bytes!("../icons/tray/bell.png").as_slice()
    };
    let icon = Image::from_bytes(icon_bytes)?;
    tray.set_icon(Some(icon))?;

    // 2. Build tray menu
    let headline_text = if state.offline {
        "Can't check your repos".to_string()
    } else if has_news_count == 0 {
        "All quiet".to_string()
    } else {
        format!(
            "{} repo{} {} news",
            has_news_count,
            if has_news_count == 1 { "" } else { "s" },
            if has_news_count == 1 { "has" } else { "have" }
        )
    };

    let headline = MenuItem::with_id(app, "headline", &headline_text, false, None::<&str>)?;
    let mut menu_builder = MenuBuilder::new(app).item(&headline).separator();

    // Repos with incoming news or errors (NO repos that only have unsaved changes)
    let mut active_repos: Vec<&RepoCardView> = state
        .repos
        .iter()
        .filter(|r| !r.ignored && (r.behind > 0 || r.error.is_some()))
        .collect();

    // Sort: highest incoming commits first, then errors
    active_repos.sort_by(|a, b| b.behind.cmp(&a.behind));

    let top_repos: Vec<&RepoCardView> = active_repos.iter().copied().take(5).collect();

    for r in &top_repos {
        let label = if let Some(ref _err) = r.error {
            format!("{} — Can't check", r.name)
        } else if r.behind > 0 {
            format!(
                "{} — {} new commit{}",
                r.name,
                r.behind,
                if r.behind == 1 { "" } else { "s" }
            )
        } else {
            format!("{} — Up to date", r.name)
        };

        let id = format!("repo:{}", r.path.display());
        let item = MenuItem::with_id(app, &id, &label, true, None::<&str>)?;
        menu_builder = menu_builder.item(&item);
    }

    if active_repos.len() > 5 {
        let more_count = active_repos.len() - 5;
        let more_label = format!("and {} more…", more_count);
        let more_item = MenuItem::with_id(app, "more_repos", &more_label, false, None::<&str>)?;
        menu_builder = menu_builder.item(&more_item);
    }

    if !top_repos.is_empty() {
        menu_builder = menu_builder.separator();
    }

    let menu = menu_builder
        .text("open", "Open RepoBell")
        .text("check_now", "Check now")
        .text("mark_seen", "Mark all as seen")
        .separator()
        .text("quit", "Quit")
        .build()?;

    tray.set_menu(Some(menu))?;
    Ok(())
}
