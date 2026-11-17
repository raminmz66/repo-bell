use std::path::{Path, PathBuf};
use std::process::Command;

use repobell_core::{Blocked, GitError, MainRefOutcome, Outcome, Plan, Problem, UndoOutcome};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::store::{self, Settings};
use crate::watcher::{self, AppStateView, RepoCardView, SharedEngine};
use crate::{autostart, editors};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanView {
    /// `up_to_date`, `fast_forward`, `combine`, or a blocked reason: `detached`, `no_upstream`,
    /// `in_progress`, `side_branch`.
    pub kind: String,
    #[serde(default)]
    pub downloads: u32,
    #[serde(default)]
    pub yours: u32,
    #[serde(default)]
    pub set_aside: bool,
    /// The one-line preview shown before the user confirms.
    #[serde(default)]
    pub message: String,
}

fn plural(n: u32) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

impl PlanView {
    pub fn from_plan(plan: &Plan, branch: Option<&str>) -> Self {
        let (kind, downloads, yours, set_aside) = match plan {
            Plan::UpToDate => ("up_to_date", 0, 0, false),
            Plan::FastForward {
                downloads,
                set_aside,
            } => ("fast_forward", *downloads, 0, *set_aside),
            Plan::Combine {
                downloads,
                yours,
                set_aside,
            } => ("combine", *downloads, *yours, *set_aside),
            Plan::Blocked(Blocked::Detached) => ("detached", 0, 0, false),
            Plan::Blocked(Blocked::NoUpstream) => ("no_upstream", 0, 0, false),
            Plan::Blocked(Blocked::InProgress) => ("in_progress", 0, 0, false),
            Plan::Blocked(Blocked::SideBranch) => ("side_branch", 0, 0, false),
        };
        let aside = if set_aside {
            ". Your unsaved changes are set aside and put back"
        } else {
            ""
        };
        let message = match plan {
            Plan::UpToDate => "Already up to date".to_string(),
            Plan::FastForward { .. } => {
                format!("This will download {downloads} change{}{aside}", plural(downloads))
            }
            Plan::Combine { .. } => format!(
                "This will download {downloads} change{} and combine them with your {yours} change{}{aside}",
                plural(downloads),
                plural(yours)
            ),
            Plan::Blocked(Blocked::Detached) => {
                "This repo isn't on a branch right now, so it can't be updated".to_string()
            }
            Plan::Blocked(Blocked::NoUpstream) => {
                "This branch isn't linked to the team's version, so it can't be updated".to_string()
            }
            Plan::Blocked(Blocked::InProgress) => {
                "Another update is half-finished in this repo. Ask a developer to finish it first".to_string()
            }
            Plan::Blocked(Blocked::SideBranch) => match branch {
                Some(b) => format!("You're on {b}, not main. Use Update main to bring main up to date"),
                None => "You're not on main. Use Update main to bring main up to date".to_string(),
            },
        };
        PlanView {
            kind: kind.to_string(),
            downloads,
            yours,
            set_aside,
            message,
        }
    }

    /// `None` for a kind we don't know.
    pub fn to_plan(&self) -> Option<Plan> {
        let (downloads, yours, set_aside) = (self.downloads, self.yours, self.set_aside);
        Some(match self.kind.as_str() {
            "up_to_date" => Plan::UpToDate,
            "fast_forward" => Plan::FastForward {
                downloads,
                set_aside,
            },
            "combine" => Plan::Combine {
                downloads,
                yours,
                set_aside,
            },
            "detached" => Plan::Blocked(Blocked::Detached),
            "no_upstream" => Plan::Blocked(Blocked::NoUpstream),
            "in_progress" => Plan::Blocked(Blocked::InProgress),
            "side_branch" => Plan::Blocked(Blocked::SideBranch),
            _ => return None,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateResult {
    /// `updated`, `up_to_date`, `conflict`, `plan_changed`, `blocked` or `edits_in_stash`.
    pub status: String,
    pub message: String,
    pub downloads: u32,
    pub conflict_files: Vec<String>,
    /// For `plan_changed`: the new plan to show and ask about again.
    pub plan: Option<PlanView>,
    /// For `edits_in_stash`: the stash entry that holds the user's changes (e.g. `stash@{1}`;
    /// the commit sha if the entry can't be found). Show it only behind "Details".
    pub stash: Option<String>,
    /// For `edits_in_stash`: the command that brings the changes back. Behind "Details" only.
    pub stash_command: Option<String>,
}

impl UpdateResult {
    fn new(status: &str, message: impl Into<String>) -> Self {
        UpdateResult {
            status: status.to_string(),
            message: message.into(),
            downloads: 0,
            conflict_files: Vec::new(),
            plan: None,
            stash: None,
            stash_command: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleResult {
    pub status: String,
    pub message: String,
    /// Git's own words, for a "details" view. Only set when something failed.
    pub detail: Option<String>,
}

impl SimpleResult {
    fn new(status: &str, message: &str) -> Self {
        SimpleResult {
            status: status.into(),
            message: message.into(),
            detail: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuggestedFolder {
    pub path: PathBuf,
    pub display: String,
    /// Full paths of the repos found.
    pub repos: Vec<PathBuf>,
}

/// True when `path` is one of the repos RepoBell is watching.
pub fn is_watched(repos: &[RepoCardView], path: &Path) -> bool {
    repos.iter().any(|r| !r.ignored && r.path == path)
}

/// Joins a repo-relative `file` onto `repo` and returns its real path, only when that path is
/// inside the repo (symlinks and `..` included).
pub fn resolve_in_repo(repo: &Path, file: &str) -> Result<PathBuf, String> {
    let rel = Path::new(file);
    if file.is_empty() || rel.is_absolute() {
        return Err("That file isn't in this repo.".to_string());
    }
    let root = repo
        .canonicalize()
        .map_err(|_| "This repo's folder wasn't found.".to_string())?;
    let full = root
        .join(rel)
        .canonicalize()
        .map_err(|_| "That file wasn't found.".to_string())?;
    if full.starts_with(&root) && full != root {
        Ok(full)
    } else {
        Err("That file isn't in this repo.".to_string())
    }
}

async fn ensure_watched(engine: &SharedEngine, path: &Path) -> Result<(), String> {
    let st = engine.state.lock().await;
    if is_watched(&st.repos, path) {
        Ok(())
    } else {
        Err("RepoBell isn't watching that repo.".to_string())
    }
}

async fn branch_of(engine: &SharedEngine, path: &Path) -> Option<String> {
    let st = engine.state.lock().await;
    st.repos
        .iter()
        .find(|r| r.path == path)
        .and_then(|r| r.branch.clone())
}

fn git_error_message(e: &GitError, repo: &Path, what: &str) -> String {
    match repobell_core::classify(e, repo) {
        Problem::Other(_) => {
            format!("Something went wrong, so {what} stopped. Ask a developer to take a look.")
        }
        p => p.message().to_string(),
    }
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|_| "Something went wrong inside RepoBell.".to_string())
}

/// Counts a running repo action, so Install waits for it. Bind it (`let _action = ...`) as the
/// first line of the command: `let _ =` would drop it at once.
pub struct ActionGuard {
    app: tauri::AppHandle,
    engine: SharedEngine,
}

/// Fails while an update downloads or installs; else counts one more running action.
pub async fn begin_action(
    app: &tauri::AppHandle,
    engine: &SharedEngine,
) -> Result<ActionGuard, String> {
    let mut st = engine.state.lock().await;
    st.update.action_allowed()?;
    st.actions_running += 1;
    watcher::publish(app, &mut st);
    drop(st);
    Ok(ActionGuard {
        app: app.clone(),
        engine: engine.clone(),
    })
}

impl Drop for ActionGuard {
    fn drop(&mut self) {
        let (app, engine) = (self.app.clone(), self.engine.clone());
        tauri::async_runtime::spawn(async move {
            let mut st = engine.state.lock().await;
            st.actions_running = st.actions_running.saturating_sub(1);
            watcher::publish(&app, &mut st);
        });
    }
}

#[tauri::command]
pub async fn get_app_state(engine: State<'_, SharedEngine>) -> Result<AppStateView, String> {
    Ok(engine.state.lock().await.to_view())
}

/// Runs a check and returns the new state. When a check is already running, this returns the
/// current state at once (with `checking: true`) and one more check runs when that one ends;
/// the result arrives as a `state-updated` event.
#[tauri::command]
pub async fn check_now(
    app: tauri::AppHandle,
    engine: State<'_, SharedEngine>,
) -> Result<AppStateView, String> {
    watcher::run_check(&app, engine.inner()).await;
    Ok(engine.state.lock().await.to_view())
}

#[tauri::command]
pub async fn mark_all_seen(
    app: tauri::AppHandle,
    engine: State<'_, SharedEngine>,
) -> Result<AppStateView, String> {
    watcher::mark_all_seen(&app, engine.inner()).await
}

async fn apply_settings(
    app: &tauri::AppHandle,
    engine: &SharedEngine,
    mut settings: Settings,
) -> Result<AppStateView, String> {
    // Any save means setup is over; emptying the folder list later must not bring the wizard back.
    settings.setup_done = true;
    // The folder walk runs before the lock is taken.
    let s = settings.clone();
    let paths = blocking(move || watcher::discover_paths(&s)).await?;
    let (view, autostart_error) = {
        let mut st = engine.state.lock().await;
        store::save_settings(&store::settings_path(), &settings)
            .map_err(|e| format!("Couldn't save your settings: {e}"))?;
        let autostart_error = autostart::set_autostart(settings.start_at_login).err();
        st.repos = watcher::merge_skeleton(paths, &settings, &st.repos);
        st.settings = settings;
        st.generation += 1;
        st.failures = 0;
        watcher::publish(app, &mut st);
        (st.to_view(), autostart_error)
    };
    let (app2, engine2) = (app.clone(), engine.clone());
    tauri::async_runtime::spawn(async move { watcher::run_check(&app2, &engine2).await });
    engine.wake.notify_one();
    match autostart_error {
        Some(e) => Err(format!(
            "Your settings were saved, but \"start at login\" couldn't be changed: {e}"
        )),
        None => Ok(view),
    }
}

#[tauri::command]
pub async fn save_settings(
    app: tauri::AppHandle,
    engine: State<'_, SharedEngine>,
    settings: Settings,
) -> Result<AppStateView, String> {
    apply_settings(&app, engine.inner(), settings).await
}

/// Removes a watched folder (for a "Folder not found" card).
#[tauri::command]
pub async fn remove_folder(
    app: tauri::AppHandle,
    engine: State<'_, SharedEngine>,
    path: PathBuf,
) -> Result<AppStateView, String> {
    let mut settings = engine.state.lock().await.settings.clone();
    settings.folders.retain(|f| *f != path);
    apply_settings(&app, engine.inner(), settings).await
}

/// What Update would do right now, as a one-line preview. Nothing changes.
#[tauri::command]
pub async fn preview_update(
    path: PathBuf,
    engine: State<'_, SharedEngine>,
) -> Result<PlanView, String> {
    ensure_watched(&engine, &path).await?;
    let branch = branch_of(&engine, &path).await;
    let _guard = engine.locks.get(&path).lock_owned().await;
    let p = path.clone();
    let plan = blocking(move || repobell_core::plan(&p)).await?;
    let plan = plan.map_err(|e| git_error_message(&e, &path, "the preview"))?;
    Ok(PlanView::from_plan(&plan, branch.as_deref()))
}

/// Runs the update the user agreed to. `expected` is the plan `preview_update` returned; when
/// the repo has changed since, nothing runs and the result is `plan_changed` with the new plan.
#[tauri::command]
pub async fn update_repo(
    app: tauri::AppHandle,
    path: PathBuf,
    expected: PlanView,
    engine: State<'_, SharedEngine>,
) -> Result<UpdateResult, String> {
    let _action = begin_action(&app, engine.inner()).await?;
    ensure_watched(&engine, &path).await?;
    let expected_plan = expected
        .to_plan()
        .ok_or_else(|| "RepoBell didn't understand that request.".to_string())?;
    let branch = branch_of(&engine, &path).await;
    let _guard = engine.locks.get(&path).lock_owned().await;
    let p = path.clone();
    let outcome = blocking(move || repobell_core::update(&p, &expected_plan)).await?;
    let engine = engine.inner();
    match outcome {
        Ok(Outcome::Updated { downloads, undo }) => {
            {
                // The new undo point changes what a running check may keep, so bump `generation`
                // in the same lock: a stale check then can't drop the point.
                let mut st = engine.state.lock().await;
                st.undo.insert(path.clone(), undo);
                st.generation += 1;
            }
            watcher::refresh_one(&app, engine, &path).await;
            let mut r = UpdateResult::new(
                "updated",
                format!(
                    "Updated. {downloads} change{} downloaded.",
                    plural(downloads)
                ),
            );
            r.downloads = downloads;
            Ok(r)
        }
        Ok(Outcome::UpToDate) => Ok(UpdateResult::new("up_to_date", "Already up to date.")),
        Ok(Outcome::Conflict { files }) => {
            let mut r = UpdateResult::new("conflict", "Update stopped. Nothing was changed.");
            r.conflict_files = files;
            Ok(r)
        }
        Ok(Outcome::PlanChanged(plan)) => {
            let view = PlanView::from_plan(&plan, branch.as_deref());
            let mut r = UpdateResult::new(
                "plan_changed",
                "This repo changed while you were looking. Check the new preview.",
            );
            r.plan = Some(view);
            Ok(r)
        }
        Ok(Outcome::Blocked(b)) => {
            let view = PlanView::from_plan(&Plan::Blocked(b), branch.as_deref());
            Ok(UpdateResult::new("blocked", view.message))
        }
        Err(GitError::EditsInStash { stash, moved, .. }) => {
            let (p, sha) = (path.clone(), stash.clone());
            let found = blocking(move || repobell_core::stash_ref(&p, &sha)).await?;
            let command = match found.as_deref() {
                None | Some("stash@{0}") => "git stash pop".to_string(),
                Some(r) => format!("git stash pop {r}"),
            };
            if moved {
                // The update went through; only putting the edits back failed.
                watcher::refresh_one(&app, engine, &path).await;
            }
            let message = if moved {
                "Updated, but your unsaved changes were put aside safely. Ask a developer to bring them back."
            } else {
                "Update stopped. Your unsaved changes were put aside safely. Ask a developer to bring them back."
            };
            let mut r = UpdateResult::new("edits_in_stash", message);
            r.stash = Some(found.unwrap_or(stash));
            r.stash_command = Some(command);
            Ok(r)
        }
        Err(e) => Err(git_error_message(&e, &path, "the update")),
    }
}

/// Undoes the last update RepoBell did in this repo. The undo point is kept by RepoBell, not
/// sent by the window.
#[tauri::command]
pub async fn undo_repo(
    app: tauri::AppHandle,
    path: PathBuf,
    engine: State<'_, SharedEngine>,
) -> Result<SimpleResult, String> {
    let _action = begin_action(&app, engine.inner()).await?;
    ensure_watched(&engine, &path).await?;
    let _guard = engine.locks.get(&path).lock_owned().await;
    let Some(point) = engine.state.lock().await.undo.get(&path).cloned() else {
        return Ok(SimpleResult::new(
            "not_possible",
            "There's nothing to undo.",
        ));
    };
    let p = path.clone();
    let outcome = blocking(move || repobell_core::undo(&p, &point)).await?;
    let engine = engine.inner();
    match outcome {
        Ok(UndoOutcome::Undone) => {
            engine.state.lock().await.undo.remove(&path);
            watcher::refresh_one(&app, engine, &path).await;
            Ok(SimpleResult::new(
                "undone",
                "Undone. The repo is back where it was.",
            ))
        }
        Ok(UndoOutcome::NotPossible) => {
            watcher::refresh_one(&app, engine, &path).await;
            Ok(SimpleResult::new(
                "not_possible",
                "Undo isn't possible because the repo changed since the update. Nothing was changed.",
            ))
        }
        Err(e) => Err(git_error_message(&e, &path, "undo")),
    }
}

#[derive(Clone, Copy)]
enum CleanupVerb {
    Delete,
    Undo,
}

fn cleanup_message(done: usize, skipped: &[String], verb: CleanupVerb) -> String {
    let branches = |n: usize| format!("{n} branch{}", if n == 1 { "" } else { "es" });
    let mut msg = match verb {
        CleanupVerb::Delete if done == 0 => {
            return "Nothing was deleted. These branches changed or are already gone.".into();
        }
        CleanupVerb::Delete => format!("Deleted {}. You can undo it.", branches(done)),
        CleanupVerb::Undo if done == 0 => "Nothing was brought back.".to_string(),
        CleanupVerb::Undo => format!("Brought back {}.", branches(done)),
    };
    match (skipped, verb) {
        ([], _) => {}
        ([one], CleanupVerb::Delete) => {
            msg += &format!(" Skipped {one} because it changed or is already gone.");
        }
        ([one], CleanupVerb::Undo) => {
            msg += &format!(" Skipped {one} because a branch with that name exists again.");
        }
        (many, CleanupVerb::Delete) => {
            msg += &format!(
                " Skipped {} because they changed or are already gone: {}.",
                branches(many.len()),
                many.join(", ")
            );
        }
        (many, CleanupVerb::Undo) => {
            msg += &format!(
                " Skipped {} because branches with those names exist again: {}.",
                branches(many.len()),
                many.join(", ")
            );
        }
    }
    msg
}

/// The repo card's main branch name ("main" when the repo isn't known yet).
async fn main_branch_of(engine: &SharedEngine, path: &Path) -> String {
    let st = engine.state.lock().await;
    st.repos
        .iter()
        .find(|r| r.path == path)
        .map(|r| r.main_branch.clone())
        .unwrap_or_else(|| "main".to_string())
}

/// Asks the server which branches it has, then refreshes the card. The network call runs with
/// no lock held; the repo lock is only taken for the local refresh.
#[tauri::command]
pub async fn refresh_old_branches(
    app: tauri::AppHandle,
    path: PathBuf,
    engine: State<'_, SharedEngine>,
) -> Result<(), String> {
    ensure_watched(&engine, &path).await?;
    let engine = engine.inner();
    let main = main_branch_of(engine, &path).await;
    let mut local = watcher::one_entry(&engine.state.lock().await.cleanup.server, &path);
    let (p, now) = (path.clone(), watcher::now_seconds());
    let local = blocking(move || {
        watcher::refresh_server_branches(&p, &main, &mut local, now, true);
        local
    })
    .await?;
    if let Some(entry) = local.get(&path) {
        engine
            .state
            .lock()
            .await
            .cleanup
            .server
            .insert(path.clone(), entry.clone());
    }
    let _guard = engine.locks.get(&path).lock_owned().await;
    watcher::refresh_one(&app, engine, &path).await;
    Ok(())
}

#[derive(Deserialize)]
pub struct BranchPick {
    pub name: String,
    pub sha: String,
}

/// Deletes the picked old branches (only those still unchanged) and keeps what is needed to
/// undo it.
#[tauri::command]
pub async fn delete_old_branches(
    app: tauri::AppHandle,
    path: PathBuf,
    branches: Vec<BranchPick>,
    engine: State<'_, SharedEngine>,
) -> Result<SimpleResult, String> {
    let _action = begin_action(&app, engine.inner()).await?;
    ensure_watched(&engine, &path).await?;
    let engine = engine.inner();
    let _guard = engine.locks.get(&path).lock_owned().await;
    let main = main_branch_of(engine, &path).await;
    let Some((server, asked)) = engine
        .state
        .lock()
        .await
        .cleanup
        .server
        .get(&path)
        .map(|s| (s.heads.clone(), s.asked.clone()))
    else {
        return Ok(SimpleResult::new(
            "not_possible",
            "RepoBell hasn't checked the server yet. Refresh the list and try again.",
        ));
    };
    let picks: Vec<(String, String)> = branches.into_iter().map(|b| (b.name, b.sha)).collect();
    let p = path.clone();
    let outcome =
        blocking(move || repobell_core::delete_branches(&p, &main, &picks, &server, &asked))
            .await?;
    let out = outcome.map_err(|e| git_error_message(&e, &path, "delete branches"))?;
    let done = out.deleted.len();
    if done > 0 {
        let mut st = engine.state.lock().await;
        st.cleanup.add_undo(path.clone(), out.deleted);
        st.generation += 1;
    }
    watcher::refresh_one(&app, engine, &path).await;
    let status = if done > 0 { "deleted" } else { "not_possible" };
    Ok(SimpleResult::new(
        status,
        &cleanup_message(done, &out.skipped, CleanupVerb::Delete),
    ))
}

/// Brings back the branches the last cleanup deleted in this repo.
#[tauri::command]
pub async fn undo_cleanup(
    app: tauri::AppHandle,
    path: PathBuf,
    engine: State<'_, SharedEngine>,
) -> Result<SimpleResult, String> {
    let _action = begin_action(&app, engine.inner()).await?;
    ensure_watched(&engine, &path).await?;
    let engine = engine.inner();
    let _guard = engine.locks.get(&path).lock_owned().await;
    let Some(list) = engine.state.lock().await.cleanup.undo.get(&path).cloned() else {
        return Ok(SimpleResult::new(
            "not_possible",
            "There's nothing to undo.",
        ));
    };
    let p = path.clone();
    let outcome = blocking(move || repobell_core::restore_branches(&p, &list)).await?;
    let out = outcome.map_err(|e| git_error_message(&e, &path, "undo"))?;
    {
        let mut st = engine.state.lock().await;
        st.cleanup.undo.remove(&path);
        st.generation += 1;
    }
    watcher::refresh_one(&app, engine, &path).await;
    let done = out.restored.len();
    let status = if done > 0 { "undone" } else { "not_possible" };
    Ok(SimpleResult::new(
        status,
        &cleanup_message(done, &out.skipped, CleanupVerb::Undo),
    ))
}

/// On a side branch: moves the local main branch up to the team's main. The files, the current
/// branch and unsaved changes stay as they are.
#[tauri::command]
pub async fn update_main(
    app: tauri::AppHandle,
    path: PathBuf,
    engine: State<'_, SharedEngine>,
) -> Result<SimpleResult, String> {
    let _action = begin_action(&app, engine.inner()).await?;
    ensure_watched(&engine, &path).await?;
    let _guard = engine.locks.get(&path).lock_owned().await;
    let p = path.clone();
    let outcome = blocking(move || repobell_core::update_main_ref(&p)).await?;
    if outcome.is_ok() {
        watcher::refresh_one(&app, engine.inner(), &path).await;
    }
    match outcome {
        Ok(MainRefOutcome::Updated { .. }) => Ok(SimpleResult::new(
            "updated",
            "Main is up to date now. Your files and branch weren't touched.",
        )),
        Ok(MainRefOutcome::AlreadyUpToDate) => Ok(SimpleResult::new(
            "up_to_date",
            "Main is already up to date.",
        )),
        Ok(MainRefOutcome::MainHasLocalChanges) => Ok(SimpleResult::new(
            "main_has_changes",
            "Your main has changes the team doesn't have yet, so it can't be moved forward. Ask a developer to take a look.",
        )),
        Ok(MainRefOutcome::Failed(detail)) => Ok(SimpleResult {
            status: "failed".into(),
            message: "Couldn't update main. Ask a developer to take a look.".into(),
            detail: Some(detail),
        }),
        Err(e) => Err(git_error_message(&e, &path, "the update")),
    }
}

/// Opens `file` (relative to the repo) in the chosen editor.
#[tauri::command]
pub async fn open_editor(
    repo: PathBuf,
    file: String,
    engine: State<'_, SharedEngine>,
) -> Result<(), String> {
    ensure_watched(&engine, &repo).await?;
    let editor = engine.state.lock().await.settings.editor.clone();
    blocking(move || {
        let full = resolve_in_repo(&repo, &file)?;
        editors::open(editor.as_deref(), &full)
    })
    .await?
}

#[tauri::command]
pub async fn pick_folder() -> Result<Option<PathBuf>, String> {
    blocking(|| {
        let out = Command::new("zenity")
            .args([
                "--file-selection",
                "--directory",
                "--title=Select Folder to Watch",
            ])
            .output();
        match out {
            Ok(out) if out.status.success() => {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                (!s.is_empty()).then(|| PathBuf::from(s))
            }
            _ => None,
        }
    })
    .await
}

/// Editors found on this computer, then "System default app".
#[tauri::command]
pub async fn detect_editors() -> Result<Vec<String>, String> {
    blocking(|| {
        editors::detect(&editors::load_entries(&editors::app_dirs()))
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    })
    .await
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// Looks for repos in one folder (same depth and skip rules as the suggestions).
fn scan(folder: PathBuf, home: &Path) -> SuggestedFolder {
    let repos = repobell_core::discover(std::slice::from_ref(&folder), 3);
    let display = match folder.strip_prefix(home) {
        Ok(rel) if !home.as_os_str().is_empty() => format!("~/{}", rel.display()),
        _ => folder.display().to_string(),
    };
    SuggestedFolder {
        path: folder,
        display,
        repos,
    }
}

fn suggested_folders() -> Vec<SuggestedFolder> {
    let home = home_dir();
    ["projects", "code", "work", "Documents", "design"]
        .map(|d| home.join(d))
        .into_iter()
        .filter(|f| f.is_dir())
        .map(|f| scan(f, &home))
        .filter(|g| !g.repos.is_empty())
        .collect()
}

#[tauri::command]
pub async fn discover_suggested_folders() -> Result<Vec<SuggestedFolder>, String> {
    blocking(suggested_folders).await
}

/// Looks for repos in one folder the user added by hand.
#[tauri::command]
pub async fn scan_folder(path: PathBuf) -> Result<SuggestedFolder, String> {
    if !path.is_dir() {
        return Err("That folder doesn't exist.".into());
    }
    blocking(move || scan(path, &home_dir())).await
}

/// Whether the top-bar bell can show: `enabled`, `disabled`, or `unknown` (not GNOME, or the
/// check isn't possible, so the window never blocks on it).
#[tauri::command]
pub async fn check_tray() -> Result<String, String> {
    blocking(|| {
        let gnome = std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.contains("GNOME"));
        if !gnome {
            return "unknown";
        }
        for uuid in [
            "ubuntu-appindicators@ubuntu.com",
            "appindicatorsupport@rgcjonas.gmail.com",
        ] {
            match Command::new("gnome-extensions")
                .args(["info", uuid])
                .output()
            {
                Err(_) => return "unknown",
                Ok(out) if extension_enabled(&String::from_utf8_lossy(&out.stdout)) => {
                    return "enabled";
                }
                Ok(_) => {}
            }
        }
        // A missing extension counts as disabled; the help text covers both.
        "disabled"
    })
    .await
    .map(str::to_string)
}

/// Reads `gnome-extensions info` output: true when its State line says ENABLED or ACTIVE.
fn extension_enabled(info: &str) -> bool {
    info.lines()
        .find_map(|l| l.trim().strip_prefix("State:"))
        .is_some_and(|s| matches!(s.trim(), "ENABLED" | "ACTIVE"))
}

#[tauri::command]
pub async fn hide_window(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(path: &str, ignored: bool) -> RepoCardView {
        RepoCardView {
            path: PathBuf::from(path),
            ignored,
            ..Default::default()
        }
    }

    #[test]
    fn reads_extension_state() {
        assert!(extension_enabled("  Name: X\n  State: ENABLED\n"));
        assert!(extension_enabled("  State: ACTIVE"));
        assert!(!extension_enabled("  State: DISABLED"));
        assert!(!extension_enabled(""));
    }

    #[test]
    fn only_watched_repos_pass() {
        let repos = [repo("/w/a", false), repo("/w/b", true)];
        assert!(is_watched(&repos, Path::new("/w/a")));
        assert!(
            !is_watched(&repos, Path::new("/w/b")),
            "ignored repos aren't watched"
        );
        assert!(!is_watched(&repos, Path::new("/w/a/../c")));
        assert!(!is_watched(&repos, Path::new("/etc")));
    }

    #[test]
    fn files_must_stay_inside_the_repo() {
        let dir = tempfile::TempDir::new().unwrap();
        let repo = dir.path().join("repo");
        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/a.txt"), "x").unwrap();
        std::fs::write(dir.path().join("secret.txt"), "x").unwrap();
        std::os::unix::fs::symlink(dir.path().join("secret.txt"), repo.join("link.txt")).unwrap();

        let ok = resolve_in_repo(&repo, "src/a.txt").unwrap();
        assert!(ok.ends_with("repo/src/a.txt"));
        assert!(resolve_in_repo(&repo, "../secret.txt").is_err());
        assert!(resolve_in_repo(&repo, "src/../../secret.txt").is_err());
        assert!(
            resolve_in_repo(&repo, "link.txt").is_err(),
            "symlink out of the repo"
        );
        assert!(resolve_in_repo(&repo, "/etc/passwd").is_err());
        assert!(resolve_in_repo(&repo, "").is_err());
        assert!(resolve_in_repo(&repo, "missing.txt").is_err());
        assert!(resolve_in_repo(&repo, ".").is_err());
    }

    #[test]
    fn plans_round_trip_and_read_plainly() {
        let plans = [
            Plan::UpToDate,
            Plan::FastForward {
                downloads: 4,
                set_aside: false,
            },
            Plan::FastForward {
                downloads: 1,
                set_aside: true,
            },
            Plan::Combine {
                downloads: 2,
                yours: 1,
                set_aside: false,
            },
            Plan::Blocked(Blocked::Detached),
            Plan::Blocked(Blocked::NoUpstream),
            Plan::Blocked(Blocked::InProgress),
            Plan::Blocked(Blocked::SideBranch),
        ];
        for p in plans {
            assert_eq!(PlanView::from_plan(&p, None).to_plan(), Some(p));
        }
        let msg = |p| PlanView::from_plan(&p, Some("fix-nav")).message;
        assert_eq!(
            msg(Plan::FastForward {
                downloads: 4,
                set_aside: false
            }),
            "This will download 4 changes"
        );
        assert_eq!(
            msg(Plan::FastForward {
                downloads: 1,
                set_aside: true
            }),
            "This will download 1 change. Your unsaved changes are set aside and put back"
        );
        assert!(msg(Plan::Blocked(Blocked::SideBranch)).starts_with("You're on fix-nav"));
        let unknown = PlanView {
            kind: "nope".into(),
            downloads: 0,
            yours: 0,
            set_aside: false,
            message: String::new(),
        };
        assert_eq!(unknown.to_plan(), None);
    }
    #[test]
    fn cleanup_messages() {
        let none: [String; 0] = [];
        let v = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        use CleanupVerb::{Delete, Undo};
        assert_eq!(
            cleanup_message(1, &none, Delete),
            "Deleted 1 branch. You can undo it."
        );
        assert_eq!(
            cleanup_message(3, &none, Delete),
            "Deleted 3 branches. You can undo it."
        );
        assert_eq!(
            cleanup_message(2, &v(&["feat-x"]), Delete),
            "Deleted 2 branches. You can undo it. Skipped feat-x because it changed or is already gone."
        );
        assert_eq!(
            cleanup_message(1, &v(&["a", "b"]), Delete),
            "Deleted 1 branch. You can undo it. Skipped 2 branches because they changed or are already gone: a, b."
        );
        assert_eq!(
            cleanup_message(0, &v(&["a"]), Delete),
            "Nothing was deleted. These branches changed or are already gone."
        );
        assert_eq!(cleanup_message(3, &none, Undo), "Brought back 3 branches.");
        assert_eq!(
            cleanup_message(1, &v(&["feat-x"]), Undo),
            "Brought back 1 branch. Skipped feat-x because a branch with that name exists again."
        );
    }
}
