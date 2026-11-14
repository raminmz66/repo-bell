use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use repobell_core::{DeletedBranch, UndoPoint};
use serde::{Deserialize, Serialize};
use tauri::async_runtime::Mutex;
use tauri::image::Image;
use tauri::menu::{MenuBuilder, MenuItem};
use tauri::tray::TrayIcon;
use tauri::{AppHandle, Emitter};
use tokio::sync::Notify;

use crate::news::{self, TrayIcon as IconKind, TrayModel};
use crate::notify;
use crate::store::{self, Notified, Seen, Settings};

/// How many folders deep repos are searched for.
const DISCOVER_DEPTH: usize = 4;
/// How many changes per repo are read, to count teammate changes. Older ones are counted
/// as teammate changes without being read.
const FEED_LIMIT: usize = 200;
/// How many changes the card lists.
const SHOWN_CHANGES: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub newest_time: Option<i64>,
    pub changes: Vec<ChangeView>,
    pub files_changed: Vec<String>,
}

/// A local branch the team deleted on the server.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OldBranchView {
    pub name: String,
    pub sha: String,
    pub merged: bool,
    pub last_commit_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RepoCardView {
    pub path: PathBuf,
    pub name: String,
    pub folder: PathBuf,
    pub main_branch: String,
    /// The checked-out branch; `None` when HEAD is detached.
    pub branch: Option<String>,
    /// True when the checked-out branch is the main branch (so Update is possible).
    pub on_main: bool,
    /// Plain-words reason the repo can't be checked.
    pub error: Option<String>,
    /// `offline`, `login`, `not_found`, `folder_missing`, `not_a_repo` or `other`.
    pub error_kind: Option<String>,
    pub behind: u32,
    pub ahead: u32,
    pub unsaved: bool,
    /// On a side branch: how many team commits the local main branch lacks ("Update main" does
    /// something only when this is above 0). Always 0 on main.
    pub main_behind: u32,
    pub muted: bool,
    pub ignored: bool,
    /// Teammate changes since the user last looked ("3 new"). This is the news number.
    pub new_count: u32,
    pub feed: Option<FeedSummary>,
    /// True while an update done from RepoBell can still be undone.
    pub can_undo: bool,
    /// Local branches the team deleted on the server.
    pub old_branches: Vec<OldBranchView>,
    /// True while a branch cleanup done from RepoBell can still be undone.
    pub can_undo_cleanup: bool,
    /// Teammate changes (newest first), for notifications. Not sent to the window.
    #[serde(skip)]
    pub teammate: Vec<ChangeView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderError {
    pub path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStateView {
    pub repos: Vec<RepoCardView>,
    pub settings: Settings,
    /// Unix seconds of the last finished check.
    pub last_checked: Option<u64>,
    /// True when every checked repo failed because the server can't be reached (no internet).
    pub offline: bool,
    pub first_run: bool,
    /// True while a check is running.
    pub checking: bool,
    /// The window's headline; the same rule as the tray headline.
    pub headline: String,
    pub news_repos: usize,
    pub news_changes: usize,
    pub error_repos: usize,
    /// Watched folders that no longer exist.
    pub folder_errors: Vec<FolderError>,
}

/// What the server said about a repo's branches, and when.
#[derive(Debug, Clone)]
pub struct ServerBranches {
    pub checked_at: u64,
    pub heads: BTreeSet<String>,
    /// The server refs the answer covers: the candidates' upstreams when it was asked.
    pub asked: BTreeSet<String>,
}

/// How long a server branch list is trusted before it is asked for again.
pub const SERVER_BRANCHES_MAX_AGE: u64 = 3600;

#[derive(Default, Clone)]
pub struct Cleanup {
    pub server: BTreeMap<PathBuf, ServerBranches>,
    /// What the last cleanup deleted, per repo, so it can be undone.
    pub undo: BTreeMap<PathBuf, Vec<DeletedBranch>>,
}

impl Cleanup {
    /// Adds a delete batch to the repo's Undo list, newest first.
    pub fn add_undo(&mut self, path: PathBuf, mut deleted: Vec<DeletedBranch>) {
        let entry = self.undo.entry(path).or_default();
        deleted.append(entry);
        *entry = deleted;
    }
}

pub struct EngineState {
    pub settings: Settings,
    pub seen: Seen,
    pub notified: Notified,
    pub repos: Vec<RepoCardView>,
    /// When a check last worked. Not set by a check that couldn't reach any server.
    pub last_checked: Option<u64>,
    /// When a check last finished, worked or not. Only the scheduler reads it.
    pub last_attempt: Option<u64>,
    pub offline: bool,
    /// Undo point of the last update RepoBell did, per repo.
    pub undo: BTreeMap<PathBuf, UndoPoint>,
    pub cleanup: Cleanup,
    /// Bumped whenever something a running check read has changed (settings, seen, an update).
    /// A check that started under an older number is thrown away.
    pub generation: u64,
    /// Checks in a row where no server could be reached.
    pub failures: u32,
    pub checking: bool,
    pub tray: Option<TrayModel>,
}

/// One lock per repo, so a check never runs Git on a repo while an update or undo does.
#[derive(Default)]
pub struct RepoLocks(StdMutex<HashMap<PathBuf, Arc<Mutex<()>>>>);

impl RepoLocks {
    pub fn get(&self, path: &Path) -> Arc<Mutex<()>> {
        let mut map = self.0.lock().unwrap_or_else(|e| e.into_inner());
        map.entry(path.to_path_buf()).or_default().clone()
    }
}

#[derive(Default)]
struct Flags {
    running: bool,
    again: bool,
}

pub struct Shared {
    pub state: Mutex<EngineState>,
    pub locks: RepoLocks,
    /// Wakes the scheduler: a check finished or the interval changed.
    pub wake: Notify,
    flags: StdMutex<Flags>,
}

pub type SharedEngine = Arc<Shared>;

impl Shared {
    pub fn new() -> SharedEngine {
        Arc::new(Shared {
            state: Mutex::new(EngineState::new()),
            locks: RepoLocks::default(),
            wake: Notify::new(),
            flags: StdMutex::new(Flags::default()),
        })
    }

    fn flags(&self) -> std::sync::MutexGuard<'_, Flags> {
        self.flags.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineState {
    /// Reads the saved files and lists repos from disk. No Git runs here: the first check
    /// happens in the background.
    pub fn new() -> Self {
        let settings = store::load_settings(&store::settings_path());
        let seen = store::load_seen(&store::seen_path());
        let notified = store::load_notified(&store::notified_path());
        let repos = skeleton_repositories(&settings, &[]);
        Self {
            settings,
            seen,
            notified,
            repos,
            last_checked: None,
            last_attempt: None,
            offline: false,
            undo: BTreeMap::new(),
            cleanup: Cleanup::default(),
            generation: 0,
            failures: 0,
            checking: false,
            tray: None,
        }
    }

    /// Watched folders that no longer exist.
    pub fn folder_errors(&self) -> Vec<FolderError> {
        self.settings
            .folders
            .iter()
            .filter(|f| !f.is_dir())
            .map(|f| FolderError {
                path: f.clone(),
                message: repobell_core::Problem::FolderMissing.message().to_string(),
            })
            .collect()
    }

    pub fn to_view(&self) -> AppStateView {
        let folder_errors = self.folder_errors();
        let s = news::summarize(&self.repos, folder_errors.len());
        AppStateView {
            repos: self.repos.clone(),
            settings: self.settings.clone(),
            last_checked: self.last_checked,
            offline: self.offline,
            first_run: !self.settings.setup_done,
            checking: self.checking,
            headline: news::headline(&s, self.offline),
            news_repos: s.news_repos,
            news_changes: s.news_changes,
            error_repos: s.error_repos,
            folder_errors,
        }
    }
}

pub fn apply_settings_to_repos(repos: &mut [RepoCardView], settings: &Settings) {
    for r in repos.iter_mut() {
        r.ignored = settings.ignored.contains(&r.path);
        r.muted = settings.muted.contains(&r.path);
    }
}

fn repo_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The watched folder a repo lives in (the deepest match).
fn folder_of(path: &Path, settings: &Settings) -> PathBuf {
    settings
        .folders
        .iter()
        .filter(|f| path.starts_with(f))
        .max_by_key(|f| f.components().count())
        .cloned()
        .or_else(|| path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| path.to_path_buf())
}

fn blank_view(path: PathBuf, settings: &Settings) -> RepoCardView {
    RepoCardView {
        name: repo_name(&path),
        folder: folder_of(&path, settings),
        main_branch: "main".to_string(),
        muted: settings.muted.contains(&path),
        ignored: settings.ignored.contains(&path),
        path,
        ..Default::default()
    }
}

/// Finds repos on disk without running Git. A repo already in `previous` keeps its last view.
pub fn skeleton_repositories(settings: &Settings, previous: &[RepoCardView]) -> Vec<RepoCardView> {
    merge_skeleton(discover_paths(settings), settings, previous)
}

/// The folder walk behind `skeleton_repositories`; blocking, so callers on the async runtime
/// run it on a blocking thread.
pub fn discover_paths(settings: &Settings) -> Vec<PathBuf> {
    if settings.folders.is_empty() {
        return Vec::new();
    }
    repobell_core::discover(&settings.folders, DISCOVER_DEPTH)
}

pub fn merge_skeleton(
    paths: Vec<PathBuf>,
    settings: &Settings,
    previous: &[RepoCardView],
) -> Vec<RepoCardView> {
    paths
        .into_iter()
        .map(|p| match previous.iter().find(|r| r.path == p) {
            Some(old) => {
                let mut v = old.clone();
                v.folder = folder_of(&p, settings);
                v.muted = settings.muted.contains(&p);
                v.ignored = settings.ignored.contains(&p);
                v
            }
            None => blank_view(p, settings),
        })
        .collect()
}

fn problem_kind(p: &repobell_core::Problem) -> &'static str {
    use repobell_core::Problem::*;
    match p {
        Offline => "offline",
        LoginFailed => "login",
        RemoteNotFound => "not_found",
        FolderMissing => "folder_missing",
        NotARepo => "not_a_repo",
        Other(_) => "other",
    }
}

fn set_error(v: &mut RepoCardView, e: &repobell_core::GitError) {
    let p = repobell_core::classify(e, &v.path);
    v.error = Some(p.message().to_string());
    v.error_kind = Some(problem_kind(&p).to_string());
}

pub struct Ctx<'a> {
    pub settings: &'a Settings,
    pub seen: &'a Seen,
    pub undo: &'a BTreeMap<PathBuf, UndoPoint>,
    pub cleanup: &'a Cleanup,
}

/// Asks the server which branches it has, when the repo has local branches worth asking about
/// and the saved answer is missing, old, or does not cover every candidate. Network: callers run it without the repo's lock.
/// A failed ask keeps the old answer.
pub fn refresh_server_branches(
    path: &Path,
    main_branch: &str,
    cache: &mut BTreeMap<PathBuf, ServerBranches>,
    now: u64,
    force: bool,
) {
    let Ok(asked) = repobell_core::candidate_upstreams(path, main_branch) else {
        return;
    };
    if asked.is_empty() {
        return;
    }
    let fresh = cache.get(path).is_some_and(|c| {
        now.saturating_sub(c.checked_at) <= SERVER_BRANCHES_MAX_AGE && asked.is_subset(&c.asked)
    });
    if fresh && !force {
        return;
    }
    if let Ok(heads) = repobell_core::server_branches(path) {
        cache.insert(
            path.to_path_buf(),
            ServerBranches {
                checked_at: now,
                heads,
                asked,
            },
        );
    }
}

/// Asks the server for a repo's main branch. Returns why it failed, if it did. Slow (network),
/// so callers run it without the repo's lock; `build_view` then reads the result from local data.
pub fn fetch_repo(path: &Path) -> Option<repobell_core::GitError> {
    let st = match repobell_core::status(path) {
        Ok(s) => s,
        Err(e) => return Some(e),
    };
    let main = st.main_branch.unwrap_or_else(|| "main".to_string());
    repobell_core::fetch_main(path, &main).err()
}

/// Reads one repo into a card from local data. `fetch_error` is the failure `fetch_repo` just
/// had, if any. The caller holds the repo's lock.
pub fn build_view(
    path: PathBuf,
    ctx: &Ctx,
    fetch_error: Option<&repobell_core::GitError>,
) -> RepoCardView {
    let mut v = blank_view(path, ctx.settings);
    if v.ignored {
        return v;
    }
    let st = match repobell_core::status(&v.path) {
        Ok(s) => s,
        Err(e) => {
            set_error(&mut v, &e);
            return v;
        }
    };
    if let Some(e) = fetch_error {
        set_error(&mut v, e);
    }
    v.behind = st.behind;
    v.main_behind = st.main_behind;
    v.ahead = st.ahead;
    v.unsaved = st.unsaved;
    v.branch = st.branch;
    v.on_main = st.on_main;
    if let Some(mb) = st.main_branch {
        v.main_branch = mb;
    }
    v.can_undo = ctx
        .undo
        .get(&v.path)
        .is_some_and(|p| repobell_core::can_undo(&v.path, p));
    // Shown even after a failed fetch: the list is local data plus the saved answer.
    v.can_undo_cleanup = ctx
        .cleanup
        .undo
        .get(&v.path)
        .is_some_and(|d| repobell_core::can_restore(&v.path, d));
    if let Some(server) = ctx.cleanup.server.get(&v.path) {
        if let Ok(list) =
            repobell_core::old_branches(&v.path, &v.main_branch, &server.heads, &server.asked)
        {
            v.old_branches = list
                .into_iter()
                .map(|b| OldBranchView {
                    name: b.name,
                    sha: b.sha,
                    merged: b.merged,
                    last_commit_time: b.last_commit_time,
                })
                .collect();
        }
    }
    if v.error.is_some() {
        // Errored repos are not news.
        return v;
    }
    let last_seen = ctx.seen.get(&v.path).map(String::as_str);
    match repobell_core::feed(&v.path, &v.main_branch, last_seen, FEED_LIMIT) {
        Ok(f) if f.total > 0 => fill_feed(&mut v, f),
        Ok(_) => {}
        Err(e) => set_error(&mut v, &e),
    }
    v
}

fn fill_feed(v: &mut RepoCardView, f: repobell_core::Feed) {
    let changes: Vec<ChangeView> = f
        .changes
        .iter()
        .map(|c| ChangeView {
            author: c.author_name.clone(),
            title: c.title.clone(),
            sha: c.sha.clone(),
            timestamp: c.time,
            mine: c.mine,
        })
        .collect();
    v.teammate = changes.iter().filter(|c| !c.mine).cloned().collect();
    v.new_count = news::count_new(v.teammate.len(), changes.len(), f.total);
    v.feed = Some(FeedSummary {
        total_changes: f.total,
        newest_time: changes.first().map(|c| c.timestamp),
        files_changed: f.files_changed,
        changes: changes.into_iter().take(SHOWN_CHANGES).collect(),
    });
}

/// Refreshes one repo from local data only (after an update or undo) and publishes the new
/// state to the window and tray. The caller already holds the repo's lock (lock order: repo
/// lock, then the state lock taken here).
pub async fn refresh_one(app: &AppHandle, engine: &SharedEngine, path: &Path) {
    let (settings, seen, undo, cleanup) = {
        let st = engine.state.lock().await;
        (
            st.settings.clone(),
            st.seen.clone(),
            st.undo.clone(),
            st.cleanup.clone(),
        )
    };
    let p = path.to_path_buf();
    let view = tauri::async_runtime::spawn_blocking(move || {
        build_view(
            p,
            &Ctx {
                settings: &settings,
                seen: &seen,
                undo: &undo,
                cleanup: &cleanup,
            },
            None,
        )
    })
    .await;
    if let Ok(view) = view {
        let mut st = engine.state.lock().await;
        if !view.can_undo && view.error.is_none() {
            st.undo.remove(&view.path);
        }
        if let Some(slot) = st.repos.iter_mut().find(|r| r.path == view.path) {
            *slot = view;
        }
        st.generation += 1;
        publish(app, &mut st);
    }
}

/// True when there is something to check and every checked repo failed because the server
/// can't be reached. One repo with a deleted remote or an expired login is not "offline".
pub fn is_offline(repos: &[RepoCardView]) -> bool {
    let mut checked = repos.iter().filter(|r| !r.ignored).peekable();
    checked.peek().is_some() && checked.all(|r| r.error_kind.as_deref() == Some("offline"))
}

/// Checks every watched repo one after another. Returns the cards and whether the whole check
/// was offline (see `is_offline`). Blocking: run it on a blocking thread. The network fetch runs
/// without the repo's lock, so an Update or Undo on that repo never waits for the server.
pub fn check_repositories(
    settings: &Settings,
    seen: &Seen,
    undo: &BTreeMap<PathBuf, UndoPoint>,
    cleanup: &mut Cleanup,
    locks: &RepoLocks,
) -> (Vec<RepoCardView>, bool) {
    if settings.folders.is_empty() {
        return (Vec::new(), false);
    }
    let mut views = Vec::new();
    for path in repobell_core::discover(&settings.folders, DISCOVER_DEPTH) {
        let fetch_error = if settings.ignored.contains(&path) {
            None
        } else {
            let e = fetch_repo(&path);
            // A failed fetch just showed the server can't answer: don't wait on it again.
            if e.is_none() {
                if let Ok(st) = repobell_core::status(&path) {
                    let main = st.main_branch.unwrap_or_else(|| "main".to_string());
                    refresh_server_branches(
                        &path,
                        &main,
                        &mut cleanup.server,
                        now_seconds(),
                        false,
                    );
                }
            }
            e
        };
        let lock = locks.get(&path);
        let _guard = lock.blocking_lock();
        let ctx = Ctx {
            settings,
            seen,
            undo,
            cleanup,
        };
        views.push(build_view(path, &ctx, fetch_error.as_ref()));
    }
    let offline = is_offline(&views);
    (views, offline)
}

/// Wait before the next check: the normal interval, doubled for each check in a row where
/// every repo failed, never more than 60 minutes (and never less than the interval).
pub fn next_delay(base_minutes: u32, failures: u32) -> Duration {
    let base = u64::from(base_minutes.max(1));
    let mins = (base << failures.min(10)).clamp(base, 60.max(base));
    Duration::from_secs(mins * 60)
}

pub fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn tray_image(icon: IconKind) -> tauri::Result<Image<'static>> {
    let bytes = match icon {
        IconKind::CantCheck => include_bytes!("../icons/tray/bell-off.png").as_slice(),
        IconKind::News => include_bytes!("../icons/tray/bell-dot.png").as_slice(),
        IconKind::Quiet => include_bytes!("../icons/tray/bell.png").as_slice(),
    };
    Image::from_bytes(bytes)
}

fn apply_tray(app: &AppHandle, tray: &TrayIcon, model: &TrayModel) -> tauri::Result<()> {
    tray.set_icon(Some(tray_image(model.icon)?))?;
    let headline = MenuItem::with_id(app, "headline", &model.headline, false, None::<&str>)?;
    let mut menu = MenuBuilder::new(app).item(&headline).separator();
    for (id, label) in &model.lines {
        menu = menu.item(&MenuItem::with_id(app, id, label, true, None::<&str>)?);
    }
    if let Some(more) = &model.more {
        menu = menu.item(&MenuItem::with_id(
            app,
            "more_repos",
            more,
            false,
            None::<&str>,
        )?);
    }
    if !model.lines.is_empty() {
        menu = menu.separator();
    }
    let menu = menu
        .text("open", "Open RepoBell")
        .text("check_now", "Check now")
        .text("mark_seen", "Mark all as seen")
        .separator()
        .text("quit", "Quit")
        .build()?;
    tray.set_menu(Some(menu))
}

/// Pushes the state to the tray (only when its labels or icon changed) and to the window.
pub fn publish(app: &AppHandle, st: &mut EngineState) {
    let model = news::tray_model(&st.repos, st.folder_errors().len(), st.offline);
    if st.tray.as_ref() != Some(&model) {
        if let Some(tray) = app.tray_by_id("main-tray") {
            if apply_tray(app, &tray, &model).is_ok() {
                st.tray = Some(model);
            }
        }
    }
    let _ = app.emit("state-updated", st.to_view());
}

/// Runs a check, or, when one is already running, asks it to run once more when it is done.
/// Returns without waiting in that case.
pub async fn run_check(app: &AppHandle, engine: &SharedEngine) {
    {
        let mut f = engine.flags();
        if f.running {
            f.again = true;
            return;
        }
        f.running = true;
        f.again = false;
    }
    loop {
        check_once(app, engine).await;
        let mut f = engine.flags();
        if f.again {
            f.again = false;
        } else {
            f.running = false;
            break;
        }
    }
}

async fn check_once(app: &AppHandle, engine: &SharedEngine) {
    let (settings, seen, undo, cleanup, generation) = {
        let mut st = engine.state.lock().await;
        st.checking = true;
        publish(app, &mut st);
        (
            st.settings.clone(),
            st.seen.clone(),
            st.undo.clone(),
            st.cleanup.clone(),
            st.generation,
        )
    };
    let eng = engine.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let mut cleanup = cleanup;
        let r = check_repositories(&settings, &seen, &undo, &mut cleanup, &eng.locks);
        (r, cleanup)
    })
    .await;

    let mut st = engine.state.lock().await;
    st.checking = false;
    let Ok(((repos, offline), cleanup)) = result else {
        publish(app, &mut st);
        return;
    };
    if st.generation != generation {
        // Settings or an update changed while we looked: these results are out of date.
        engine.flags().again = true;
        publish(app, &mut st);
        return;
    }
    // Keep an undo point while HEAD is still where the update left it.
    st.undo.retain(|p, _| {
        repos
            .iter()
            .any(|r| &r.path == p && (r.can_undo || r.error.is_some()))
    });
    st.cleanup.server = cleanup.server;
    st.cleanup.undo.retain(|p, _| {
        repos
            .iter()
            .any(|r| &r.path == p && (r.can_undo_cleanup || r.error.is_some()))
    });
    st.repos = repos;
    st.offline = offline;
    st.failures = if offline { st.failures + 1 } else { 0 };
    st.last_attempt = Some(now_seconds());
    if !offline {
        st.last_checked = st.last_attempt;
    }
    let (toast, changed) = {
        let s = &mut *st;
        notify::collect(&s.repos, &mut s.notified)
    };
    if changed {
        if let Err(e) = store::save_notified(&store::notified_path(), &st.notified) {
            eprintln!("repobell: could not save notified.json: {e}");
        }
    }
    publish(app, &mut st);
    drop(st);
    if let Some(t) = toast {
        notify::send(&t);
    }
    engine.wake.notify_one();
}

/// Seconds until the next check is due; zero when none has run yet.
async fn due_in(engine: &SharedEngine) -> Duration {
    let st = engine.state.lock().await;
    match st.last_attempt {
        None => Duration::ZERO,
        Some(t) => {
            let delay = next_delay(st.settings.check_minutes, st.failures).as_secs();
            Duration::from_secs((t + delay).saturating_sub(now_seconds()))
        }
    }
}

/// The background loop. It sleeps in steps of at most 60 seconds and compares wall-clock time,
/// so after the computer wakes from sleep a check runs within a minute. A finished check or a
/// settings change wakes it, so a new interval and "Check now" take effect at once.
pub async fn scheduler(app: AppHandle, engine: SharedEngine) {
    loop {
        let due = due_in(&engine).await;
        if due.is_zero() {
            run_check(&app, &engine).await;
            if due_in(&engine).await.is_zero() {
                // The check didn't finish a pass (another one was running, or results were
                // thrown away). Wait a moment instead of spinning.
                let _ = tokio::time::timeout(Duration::from_secs(5), engine.wake.notified()).await;
            }
            continue;
        }
        let _ =
            tokio::time::timeout(due.min(Duration::from_secs(60)), engine.wake.notified()).await;
    }
}

/// Sets "last seen" to the current remote main for every watched repo and clears the news.
pub async fn mark_all_seen(app: &AppHandle, engine: &SharedEngine) -> Result<AppStateView, String> {
    let targets: Vec<(PathBuf, String)> = {
        let st = engine.state.lock().await;
        st.repos
            .iter()
            .filter(|r| !r.ignored)
            .map(|r| (r.path.clone(), r.main_branch.clone()))
            .collect()
    };
    let shas = tauri::async_runtime::spawn_blocking(move || {
        targets
            .into_iter()
            .filter_map(|(p, b)| repobell_core::remote_main_sha(&p, &b).ok().map(|s| (p, s)))
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|e| e.to_string())?;

    let mut st = engine.state.lock().await;
    for (p, s) in shas {
        st.seen.insert(p, s);
    }
    let saved = store::save_seen(&store::seen_path(), &st.seen);
    let seen = st.seen.clone();
    for r in st.repos.iter_mut().filter(|r| seen.contains_key(&r.path)) {
        r.feed = None;
        r.new_count = 0;
        r.teammate.clear();
    }
    st.generation += 1;
    publish(app, &mut st);
    let view = st.to_view();
    saved
        .map(|_| view)
        .map_err(|e| format!("Couldn't save what you've seen: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_doubles_up_to_an_hour_and_resets() {
        let mins = |base, f| next_delay(base, f).as_secs() / 60;
        let run: Vec<_> = (0..6).map(|f| mins(5, f)).collect();
        assert_eq!(run, [5, 10, 20, 40, 60, 60]);
        assert_eq!(mins(30, 0), 30);
        assert_eq!(mins(30, 1), 60);
        assert_eq!(mins(60, 3), 60);
        assert_eq!(mins(1, 30), 60);
        assert_eq!(mins(0, 0), 1, "a zero interval counts as one minute");
    }

    #[test]
    fn folder_is_the_deepest_watched_match() {
        let s = Settings {
            folders: vec![PathBuf::from("/w"), PathBuf::from("/w/team")],
            ..Settings::default()
        };
        assert_eq!(
            folder_of(Path::new("/w/team/app"), &s),
            PathBuf::from("/w/team")
        );
        assert_eq!(folder_of(Path::new("/w/solo"), &s), PathBuf::from("/w"));
    }
}
