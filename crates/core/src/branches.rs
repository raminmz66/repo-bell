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
