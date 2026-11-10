use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use crate::git::{self, GitError};

/// A local branch the team already deleted on the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OldBranch {
    pub name: String,
    /// Tip commit of the branch.
    pub sha: String,
    /// True when the tip is already part of the remote main branch. A squash-merged branch is
    /// not merged in this sense.
    pub merged: bool,
    /// Commit time of the tip, in Unix seconds.
    pub last_commit_time: i64,
}

/// A local branch that passes every listing rule except the server one.
struct Candidate {
    name: String,
    sha: String,
    /// Full ref the branch tracks on the server, such as `refs/heads/feat`.
    remote_ref: String,
    last_commit_time: i64,
}

/// True when at least one local branch could be listed once the server answers.
///
/// Local only: it never talks to the server. See `old_branches` for the rules.
pub fn has_candidates(repo: &Path, main_branch: &str) -> Result<bool, GitError> {
    Ok(!candidates(repo, main_branch)?.is_empty())
}

/// The branches on `origin`, as full refs such as `refs/heads/feat` (from `git ls-remote --heads`).
///
/// Fails when there is no `origin` or it cannot be reached; callers treat that as "no answer"
/// and must not offer to delete anything.
pub fn server_branches(repo: &Path) -> Result<BTreeSet<String>, GitError> {
    let out = git::run_with_timeout(
        repo,
        &["ls-remote", "--heads", "origin"],
        &crate::fetch::ssh_env(repo),
        Duration::from_secs(60),
    )?;
    Ok(out
        .lines()
        .filter_map(|l| l.split('\t').nth(1))
        .map(str::to_string)
        .collect())
}

/// Local branches the team deleted on the server, merged ones first, then by name.
///
/// A branch is listed only when all of these hold:
/// - it tracks a branch on `origin` (a branch never pushed is never listed),
/// - that server branch is not `server` (the answer of `server_branches`) any more,
/// - it is not the main branch, does not track the main branch, and is not checked out in any
///   worktree (the main checkout included).
///
/// `merged` compares the tip with the local copy of `origin/<main_branch>`, so fetch main first.
pub fn old_branches(
    repo: &Path,
    main_branch: &str,
    server: &BTreeSet<String>,
) -> Result<Vec<OldBranch>, GitError> {
    let main_ref = format!("refs/remotes/origin/{main_branch}");
    let mut out = Vec::new();
    for c in candidates(repo, main_branch)? {
        if server.contains(&c.remote_ref) {
            continue;
        }
        let merged =
            match git::run_background(repo, &["merge-base", "--is-ancestor", &c.sha, &main_ref]) {
                Ok(_) => true,
                // Exit 1 with no message means "not an ancestor"; anything else is a real failure.
                Err(GitError::Failed { stderr, .. }) if stderr.trim().is_empty() => false,
                Err(e) => return Err(e),
            };
        out.push(OldBranch {
            name: c.name,
            sha: c.sha,
            merged,
            last_commit_time: c.last_commit_time,
        });
    }
    out.sort_by(|a, b| b.merged.cmp(&a.merged).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

/// The one place that decides which local branches may be old (everything but the server check).
fn candidates(repo: &Path, main_branch: &str) -> Result<Vec<Candidate>, GitError> {
    let refs = git::run_background(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname:strip=2)%00%(objectname)%00%(upstream:remotename)%00%(upstream:remoteref)%00%(committerdate:unix)",
            "refs/heads",
        ],
    )?;
    let worktrees = git::run_background(repo, &["worktree", "list", "--porcelain"])?;
    let checked_out: BTreeSet<&str> = worktrees
        .lines()
        .filter_map(|l| l.strip_prefix("branch refs/heads/"))
        .collect();
    let main_ref = format!("refs/heads/{main_branch}");
    Ok(refs
        .lines()
        .filter_map(|line| {
            let mut f = line.split('\0');
            let (name, sha, remote, remote_ref, time) =
                (f.next()?, f.next()?, f.next()?, f.next()?, f.next()?);
            if remote != "origin"
                || remote_ref.is_empty()
                || remote_ref == main_ref
                || name == main_branch
                || checked_out.contains(name)
            {
                return None;
            }
            Some(Candidate {
                name: name.to_string(),
                sha: sha.to_string(),
                remote_ref: remote_ref.to_string(),
                last_commit_time: time.parse().ok()?,
            })
        })
        .collect())
}

/// A branch removed by `delete_branches`, with what `restore_branches` needs to bring it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletedBranch {
    pub name: String,
    pub sha: String,
    /// The old `branch.<name>.remote` value (empty when unset).
    pub remote: String,
    /// The old `branch.<name>.merge` value (empty when unset).
    pub merge: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct DeleteOutcome {
    pub deleted: Vec<DeletedBranch>,
    /// Picks that were not deleted: not listed any more, moved, or already gone.
    pub skipped: Vec<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct RestoreOutcome {
    pub restored: Vec<String>,
    /// Names that could not be restored because the name is taken again.
    pub skipped: Vec<String>,
}

/// Reads a config value; a missing key is an empty string.
fn config_get(repo: &Path, key: &str) -> String {
    git::run(repo, &["config", "--get", key])
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Deletes the picked `(name, sha)` branches, but only those still listed by `old_branches` with
/// that same tip. Each delete checks the tip again, so a branch that moved is never lost.
pub fn delete_branches(
    repo: &Path,
    main_branch: &str,
    picks: &[(String, String)],
    server: &BTreeSet<String>,
) -> Result<DeleteOutcome, GitError> {
    let listed = old_branches(repo, main_branch, server)?;
    let mut out = DeleteOutcome::default();
    for (name, sha) in picks {
        if !listed.iter().any(|b| &b.name == name && &b.sha == sha) {
            out.skipped.push(name.clone());
            continue;
        }
        let remote = config_get(repo, &format!("branch.{name}.remote"));
        let merge = config_get(repo, &format!("branch.{name}.merge"));
        match git::run(
            repo,
            &["update-ref", "-d", &format!("refs/heads/{name}"), sha],
        ) {
            Ok(_) => {
                let _ = git::run(
                    repo,
                    &["config", "--remove-section", &format!("branch.{name}")],
                );
                out.deleted.push(DeletedBranch {
                    name: name.clone(),
                    sha: sha.clone(),
                    remote,
                    merge,
                });
            }
            // Any error counts as skipped: returning early would drop the undo data of the
            // branches already deleted.
            Err(_) => out.skipped.push(name.clone()),
        }
    }
    Ok(out)
}

/// Brings deleted branches back at their old tip with their old upstream. A name that is taken
/// again is skipped and left alone.
pub fn restore_branches(
    repo: &Path,
    deleted: &[DeletedBranch],
) -> Result<RestoreOutcome, GitError> {
    let mut out = RestoreOutcome::default();
    for d in deleted {
        // The empty old value makes update-ref create the ref only, never overwrite one.
        match git::run(
            repo,
            &["update-ref", &format!("refs/heads/{}", d.name), &d.sha, ""],
        ) {
            Ok(_) => {}
            Err(GitError::Failed { .. }) => {
                out.skipped.push(d.name.clone());
                continue;
            }
            Err(e) => return Err(e),
        }
        for (key, val) in [("remote", &d.remote), ("merge", &d.merge)] {
            if !val.is_empty() {
                git::run(repo, &["config", &format!("branch.{}.{key}", d.name), val])?;
            }
        }
        out.restored.push(d.name.clone());
    }
    Ok(out)
}

/// True when at least one deleted name is free again.
pub fn can_restore(repo: &Path, deleted: &[DeletedBranch]) -> bool {
    deleted.iter().any(|d| {
        git::run(
            repo,
            &[
                "rev-parse",
                "--verify",
                "-q",
                &format!("refs/heads/{}", d.name),
            ],
        )
        .is_err()
    })
}
