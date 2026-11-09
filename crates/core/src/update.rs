use std::path::Path;

use crate::git::{run, GitError};
use crate::status::status;

/// Why an update can't run at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Blocked {
    /// HEAD is detached (no branch checked out).
    Detached,
    /// The current branch has no upstream.
    NoUpstream,
    /// A merge, rebase, cherry-pick or revert is already in progress.
    InProgress,
    /// The current branch is not main, or it does not track `origin/<main>`.
    /// Use `update_main_ref` to move the local main branch instead.
    SideBranch,
}

/// What pressing Update would do. Shown to the user as the preview before anything runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    /// Nothing to download.
    UpToDate,
    /// Only the team changed: a fast-forward. `set_aside` is true when unsaved changes must be
    /// set aside first and brought back after.
    FastForward { downloads: u32, set_aside: bool },
    /// Both sides have commits: a normal merge. `set_aside` as above.
    Combine {
        downloads: u32,
        yours: u32,
        set_aside: bool,
    },
    /// Update is not possible.
    Blocked(Blocked),
}

/// Enough to undo an update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoPoint {
    /// HEAD before the update.
    pub before: String,
    /// HEAD right after the update.
    pub after: String,
}

/// What `update` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing to download; nothing changed.
    UpToDate,
    /// The branch moved forward. `downloads` = commits that came in.
    Updated { downloads: u32, undo: UndoPoint },
    /// The update stopped. The repo is back exactly as it was before (same HEAD, same
    /// unsaved changes, same untracked files). `files` lists the conflicting paths, sorted.
    Conflict { files: Vec<String> },
    /// The repo changed since the plan was shown (the fresh plan differs from `expected`).
    /// Nothing was done; show the new plan and ask again.
    PlanChanged(Plan),
    /// Update is not possible.
    Blocked(Blocked),
}

/// What `undo` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UndoOutcome {
    Undone,
    /// Undo would not be safe, so nothing was changed.
    NotPossible,
}

/// What `update_main_ref` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MainRefOutcome {
    /// Local main moved from `from` to `to` (full shas).
    Updated { from: String, to: String },
    /// Local main already matches (or is ahead of nothing in) `origin/<main>`.
    AlreadyUpToDate,
    /// Local main has commits the team's main doesn't have, so it can't fast-forward.
    MainHasLocalChanges,
    /// Anything else (no main known, main checked out here or in another worktree...).
    Failed(String),
}

/// Works out what Update would do, from local data only (fetch first).
/// Uses the current branch and its upstream; never looks at other branches.
/// Order of checks: detached HEAD, operation in progress, side branch (not main, or main without
/// upstream `origin/<main>`), no upstream, then behind/ahead/unsaved.
/// Untracked files count as unsaved changes.
pub fn plan(repo: &Path) -> Result<Plan, GitError> {
    let st = status(repo)?;
    if st.branch.is_none() {
        return Ok(Plan::Blocked(Blocked::Detached));
    }
    if in_progress(repo)? {
        return Ok(Plan::Blocked(Blocked::InProgress));
    }
    let side = st.main_branch.is_some() && st.branch != st.main_branch;
    if side || (st.upstream.is_some() && !st.on_main) {
        return Ok(Plan::Blocked(Blocked::SideBranch));
    }
    if st.upstream.is_none() {
        return Ok(Plan::Blocked(Blocked::NoUpstream));
    }
    let (downloads, yours, set_aside) = (st.behind, st.ahead, st.unsaved);
    Ok(if downloads == 0 {
        Plan::UpToDate
    } else if yours == 0 {
        Plan::FastForward {
            downloads,
            set_aside,
        }
    } else {
        Plan::Combine {
            downloads,
            yours,
            set_aside,
        }
    })
}

/// Runs the update the user agreed to.
///
/// Rules:
/// - First recomputes `plan`; if it differs from `expected`, returns `PlanChanged` and does nothing.
/// - Never switches branches and never touches any branch but the current one.
/// - FastForward: `git merge --ff-only <upstream sha>` (pinned once, so a fetch can't move it).
/// - Combine: `git merge --no-edit <upstream sha>`. On conflict: list the conflicting files, `git merge --abort`.
/// - set_aside: before updating, `git stash push --include-untracked` with a message starting
///   "RepoBell"; after updating, `git stash pop`. If the pop conflicts: list the conflicting files,
///   `git reset --hard <before>`, then `git stash pop` again so the user's work is back exactly as
///   it was, and return `Conflict`.
/// - Pops with `--index` (so staged stays staged), falling back to a plain pop.
/// - Lock errors (`index.lock`) are retried a few times. If the repo still can't be put back,
///   returns `GitError::EditsInStash`: the user's edits are safe in that stash.
/// - If the update would overwrite a local ignored file (such as `.env`), returns `Conflict`
///   with that file and changes nothing.
/// - Whatever fails, the repo ends where it started, and a stash RepoBell made is never dropped
///   unless it was applied cleanly (so the user's work can't be lost).
pub fn update(repo: &Path, expected: &Plan) -> Result<Outcome, GitError> {
    let fresh = plan(repo)?;
    if fresh != *expected {
        return Ok(Outcome::PlanChanged(fresh));
    }
    let (downloads, combine, set_aside) = match fresh {
        Plan::UpToDate => return Ok(Outcome::UpToDate),
        Plan::Blocked(b) => return Ok(Outcome::Blocked(b)),
        Plan::FastForward {
            downloads,
            set_aside,
        } => (downloads, false, set_aside),
        Plan::Combine {
            downloads,
            set_aside,
            ..
        } => (downloads, true, set_aside),
    };
    let before = head(repo)?;
    // Pin the target so a background fetch can't change what gets checked and merged.
    let target = run(repo, &["rev-parse", "--verify", "@{u}^{commit}"])?
        .trim()
        .to_string();
    // Git silently overwrites ignored files (a local .env) when the update adds a tracked one.
    let clashes = ignored_clashes(repo, &target)?;
    if !clashes.is_empty() {
        return Ok(Outcome::Conflict { files: clashes });
    }
    let stash = if set_aside { push_stash(repo)? } else { None };

    // Keep Git's usual merge message even though we merge a pinned sha.
    let upstream = run(repo, &["rev-parse", "--abbrev-ref", "@{u}"])?
        .trim()
        .to_string();
    let message = format!("Merge remote-tracking branch '{upstream}'");
    let merge: &[&str] = if combine {
        &["merge", "--no-edit", "-m", &message, &target]
    } else {
        &["merge", "--ff-only", &target]
    };
    if let Err(e) = retry_locked(repo, merge) {
        let undo_merge = || -> Result<Vec<String>, GitError> {
            let files = conflicts(repo)?;
            if exists(repo, "MERGE_HEAD")? {
                retry_locked(repo, &["merge", "--abort"])?;
            }
            if let Some(sha) = &stash {
                check_top(repo, sha)?;
                pop_stash(repo)?;
            }
            Ok(files)
        };
        let files = undo_merge().map_err(|r| edits_in_stash(repo, &before, &stash, r))?;
        return if files.is_empty() {
            Err(e)
        } else {
            Ok(Outcome::Conflict { files })
        };
    }

    if let Some(sha) = &stash {
        check_top(repo, sha).map_err(|r| edits_in_stash(repo, &before, &stash, r))?;
        if let Err(e) = pop_stash(repo) {
            // The stash stays until a pop applies cleanly, so the work is never lost.
            let rollback = || -> Result<Vec<String>, GitError> {
                let mut files = conflicts(repo)?;
                if let GitError::Failed { stderr, .. } = &e {
                    // An untracked file from the stash can clash with a file the team added.
                    files.extend(stderr.lines().filter_map(|l| {
                        l.trim()
                            .strip_suffix(" already exists, no checkout")
                            .map(|p| p.trim().to_string())
                    }));
                    files.sort();
                    files.dedup();
                }
                retry_locked(repo, &["reset", "--hard", &before])?;
                remove_restored_untracked(repo)?;
                pop_stash(repo)?;
                Ok(files)
            };
            let files = rollback().map_err(|r| edits_in_stash(repo, &before, &stash, r))?;
            return if files.is_empty() {
                Err(e)
            } else {
                Ok(Outcome::Conflict { files })
            };
        }
    }
    let after = head(repo)?;
    Ok(Outcome::Updated {
        downloads,
        undo: UndoPoint { before, after },
    })
}

/// True while undo is possible: HEAD is still `point.after` and no merge or rebase is in progress.
pub fn can_undo(repo: &Path, point: &UndoPoint) -> bool {
    matches!(head(repo), Ok(h) if h == point.after) && matches!(in_progress(repo), Ok(false))
}

/// Undoes an update with `git reset --keep <before>` (`before` must be a full hex sha, since `git reset` has no `--end-of-options` before Git 2.40): commits go back, unsaved changes stay.
/// Git refuses when an unsaved change would be overwritten; then nothing changes and the result
/// is `NotPossible`. Also `NotPossible` when `can_undo` is false.
pub fn undo(repo: &Path, point: &UndoPoint) -> Result<UndoOutcome, GitError> {
    let full_sha = [40, 64].contains(&point.before.len())
        && point.before.bytes().all(|b| b.is_ascii_hexdigit());
    if !full_sha || !can_undo(repo, point) {
        return Ok(UndoOutcome::NotPossible);
    }
    match run(repo, &["reset", "--keep", &point.before]) {
        Ok(_) => Ok(UndoOutcome::Undone),
        Err(GitError::Failed { .. }) => Ok(UndoOutcome::NotPossible),
        Err(e) => Err(e),
    }
}

fn head(repo: &Path) -> Result<String, GitError> {
    Ok(run(repo, &["rev-parse", "HEAD"])?.trim().to_string())
}

/// Does the Git state file `name` (such as MERGE_HEAD) exist?
fn exists(repo: &Path, name: &str) -> Result<bool, GitError> {
    let out = run(repo, &["rev-parse", "--git-path", name])?;
    Ok(repo.join(out.trim()).exists())
}

fn in_progress(repo: &Path) -> Result<bool, GitError> {
    for name in [
        "MERGE_HEAD",
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "rebase-merge",
        "rebase-apply",
    ] {
        if exists(repo, name)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn conflicts(repo: &Path) -> Result<Vec<String>, GitError> {
    let out = run(repo, &["diff", "--name-only", "--diff-filter=U"])?;
    let mut files: Vec<String> = out.lines().map(str::to_string).collect();
    files.sort();
    Ok(files)
}

/// The stash entry (like `stash@{2}`) that holds commit `sha`, or `None` if it is gone.
pub fn stash_ref(repo: &Path, sha: &str) -> Option<String> {
    let list = run(repo, &["stash", "list", "--format=%gd %H"]).ok()?;
    list.lines().find_map(|l| {
        let (name, h) = l.split_once(' ')?;
        (h.trim() == sha).then(|| name.to_string())
    })
}

fn stash_top(repo: &Path) -> Option<String> {
    run(repo, &["rev-parse", "--verify", "--quiet", "stash@{0}"])
        .ok()
        .map(|s| s.trim().to_string())
}

/// Stashes the unsaved work. Returns the new stash's sha, or `None` if Git made no stash.
fn push_stash(repo: &Path) -> Result<Option<String>, GitError> {
    let old = stash_top(repo);
    run(
        repo,
        &[
            "stash",
            "push",
            "--include-untracked",
            "-m",
            "RepoBell: set aside before update",
        ],
    )?;
    let new = stash_top(repo);
    Ok(if new == old { None } else { new })
}

/// Errors unless `stash@{0}` is still RepoBell's stash, so we never pop someone else's.
fn check_top(repo: &Path, sha: &str) -> Result<(), GitError> {
    if stash_top(repo).as_deref() == Some(sha) {
        return Ok(());
    }
    Err(GitError::Failed {
        args: "stash pop".to_string(),
        stderr: format!(
            "the stash {sha} is no longer on top; your work is still in the stash list"
        ),
    })
}

/// A failed pop may already have written the stash's untracked files back, which makes the next
/// pop refuse. Delete only files that are byte-identical to the stash's copy, so nothing is lost.
fn remove_restored_untracked(repo: &Path) -> Result<(), GitError> {
    let Ok(list) = run(repo, &["ls-tree", "-r", "--name-only", "stash@{0}^3"]) else {
        return Ok(()); // the stash holds no untracked files
    };
    for name in list.lines() {
        let in_stash = run(repo, &["rev-parse", &format!("stash@{{0}}^3:{name}")])?;
        let on_disk = run(repo, &["hash-object", "--", name]);
        if matches!(on_disk, Ok(h) if h == in_stash) {
            std::fs::remove_file(repo.join(name)).map_err(GitError::Io)?;
        }
    }
    Ok(())
}

/// Files the update would add that already exist in the folder as ignored files.
/// Renames count as adds (`--no-renames`), so a teammate's `git mv env.sample .env` is caught.
fn ignored_clashes(repo: &Path, target: &str) -> Result<Vec<String>, GitError> {
    let added = run(
        repo,
        &[
            "diff",
            "-z",
            "--name-only",
            "--no-renames",
            "--diff-filter=A",
            "HEAD",
            target,
        ],
    )?;
    // Only paths that exist here can be overwritten. Ask Git about those, a batch at a time.
    let existing: Vec<&str> = added
        .split('\0')
        .filter(|f| !f.is_empty() && repo.join(f).symlink_metadata().is_ok())
        .collect();
    let mut files = Vec::new();
    for chunk in existing.chunks(200) {
        let mut args = vec!["check-ignore", "--"];
        args.extend(chunk);
        match run(repo, &args) {
            Ok(out) => files.extend(out.lines().map(str::to_string)),
            // Exit 1 with no message means none of them is ignored.
            Err(GitError::Failed { stderr, .. }) if stderr.trim().is_empty() => {}
            Err(e) => return Err(e),
        }
    }
    files.sort();
    Ok(files)
}

const LOCK_TRIES: u32 = 5;
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_millis(300);

/// Runs a write command, retrying a few times while another Git process holds a `.lock` file.
fn retry_locked(repo: &Path, args: &[&str]) -> Result<String, GitError> {
    let mut tries = 0;
    loop {
        match run(repo, args) {
            // `stash pop` hides the lock error, so also look for the lock file itself.
            Err(GitError::Failed { stderr, .. })
                if tries < LOCK_TRIES && (stderr.contains(".lock") || locked(repo)) =>
            {
                tries += 1;
                std::thread::sleep(LOCK_WAIT);
            }
            other => return other,
        }
    }
}

fn locked(repo: &Path) -> bool {
    exists(repo, "index.lock").unwrap_or(false)
}

/// `git stash pop --index` so staged files stay staged; plain `git stash pop` if that fails for a
/// reason other than a conflict or a lock.
fn pop_stash(repo: &Path) -> Result<(), GitError> {
    match retry_locked(repo, &["stash", "pop", "--index"]) {
        Err(GitError::Failed { stderr, .. })
            if !stderr.contains(".lock")
                && !locked(repo)
                && !stderr.contains("already exists, no checkout")
                && conflicts(repo)?.is_empty() =>
        {
            retry_locked(repo, &["stash", "pop"]).map(|_| ())
        }
        r => r.map(|_| ()),
    }
}

/// Wraps a failed rollback: when RepoBell made a stash, say the edits are safe in it.
/// `moved` says whether HEAD had already left `before` (the update went through).
fn edits_in_stash(repo: &Path, before: &str, stash: &Option<String>, e: GitError) -> GitError {
    match stash {
        Some(sha) if !matches!(e, GitError::EditsInStash { .. }) => GitError::EditsInStash {
            stash: sha.clone(),
            reason: e.to_string(),
            moved: head(repo).is_ok_and(|h| h != before),
        },
        _ => e,
    }
}

/// Fast-forwards the local main branch to `origin/<main>` (already fetched) without touching the
/// worktree, the index, the current branch or HEAD. No network.
///
/// Runs `git fetch . refs/remotes/origin/<main>:refs/heads/<main>`, which Git refuses when it is
/// not a fast-forward or when main is checked out in any worktree (then `Failed`).
pub fn update_main_ref(repo: &Path) -> Result<MainRefOutcome, GitError> {
    let Some(main) = status(repo)?.main_branch else {
        return Ok(MainRefOutcome::Failed(
            "the main branch is not known".into(),
        ));
    };
    let (local, remote) = (
        format!("refs/heads/{main}"),
        format!("refs/remotes/origin/{main}"),
    );
    let (Ok(from), Ok(to)) = (
        run(repo, &["rev-parse", "--verify", &local]),
        run(repo, &["rev-parse", "--verify", &remote]),
    ) else {
        return Ok(MainRefOutcome::Failed(format!(
            "{local} or {remote} is missing"
        )));
    };
    let (from, to) = (from.trim().to_string(), to.trim().to_string());
    if run(repo, &["merge-base", "--is-ancestor", &to, &from]).is_ok() {
        return Ok(MainRefOutcome::AlreadyUpToDate);
    }
    if run(repo, &["merge-base", "--is-ancestor", &from, &to]).is_err() {
        return Ok(MainRefOutcome::MainHasLocalChanges);
    }
    let spec = format!("{remote}:{local}");
    match run(
        repo,
        &[
            "fetch",
            "--quiet",
            "--no-tags",
            "--no-write-fetch-head",
            ".",
            &spec,
        ],
    ) {
        Ok(_) => Ok(MainRefOutcome::Updated { from, to }),
        Err(GitError::Failed { stderr, .. }) => {
            Ok(MainRefOutcome::Failed(stderr.trim().to_string()))
        }
        Err(e) => Err(e),
    }
}
