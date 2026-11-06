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

/// Works out what Update would do, from local data only (fetch first).
/// Uses the current branch and its upstream; never looks at other branches.
/// Order of checks: detached HEAD, operation in progress, no upstream, then behind/ahead/unsaved.
/// Untracked files count as unsaved changes.
pub fn plan(repo: &Path) -> Result<Plan, GitError> {
    let st = status(repo)?;
    if st.branch.is_none() {
        return Ok(Plan::Blocked(Blocked::Detached));
    }
    if in_progress(repo)? {
        return Ok(Plan::Blocked(Blocked::InProgress));
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
/// - FastForward: `git merge --ff-only @{u}`.
/// - Combine: `git merge --no-edit @{u}`. On conflict: list the conflicting files, `git merge --abort`.
/// - set_aside: before updating, `git stash push --include-untracked` with a message starting
///   "RepoBell"; after updating, `git stash pop`. If the pop conflicts: list the conflicting files,
///   `git reset --hard <before>`, then `git stash pop` again so the user's work is back exactly as
///   it was, and return `Conflict`.
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
    let stash = if set_aside { push_stash(repo)? } else { None };

    let merge: &[&str] = if combine {
        &["merge", "--no-edit", "@{u}"]
    } else {
        &["merge", "--ff-only", "@{u}"]
    };
    if let Err(e) = run(repo, merge) {
        let files = conflicts(repo)?;
        if exists(repo, "MERGE_HEAD")? {
            run(repo, &["merge", "--abort"])?;
        }
        if let Some(sha) = &stash {
            check_top(repo, sha)?;
            run(repo, &["stash", "pop"])?;
        }
        return if files.is_empty() {
            Err(e)
        } else {
            Ok(Outcome::Conflict { files })
        };
    }

    if let Some(sha) = &stash {
        check_top(repo, sha)?;
        if let Err(e) = run(repo, &["stash", "pop"]) {
            // The stash stays until a pop applies cleanly, so the work is never lost.
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
            run(repo, &["reset", "--hard", &before])?;
            remove_restored_untracked(repo)?;
            run(repo, &["stash", "pop"])?;
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

/// Undoes an update with `git reset --keep <before>`: commits go back, unsaved changes stay.
/// Git refuses when an unsaved change would be overwritten; then nothing changes and the result
/// is `NotPossible`. Also `NotPossible` when `can_undo` is false.
pub fn undo(repo: &Path, point: &UndoPoint) -> Result<UndoOutcome, GitError> {
    if !can_undo(repo, point) {
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
