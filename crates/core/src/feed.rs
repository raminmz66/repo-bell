use std::path::Path;

use crate::git::GitError;

/// One commit in the feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub sha: String,
    pub author_name: String,
    pub author_email: String,
    /// The first line of the commit message.
    pub title: String,
    /// Author time in Unix seconds.
    pub time: i64,
    /// True when `author_email` equals the repo's `user.email` (case-insensitive).
    /// False when `user.email` is not set.
    pub mine: bool,
}

/// What is new on the remote main branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feed {
    /// Newest first, at most `limit` entries.
    pub changes: Vec<Change>,
    /// Number of commits in the range, even when `changes` was cut by `limit`.
    pub total: usize,
    /// Every file path touched by any commit in the range, sorted, without duplicates.
    pub files_changed: Vec<String>,
}

/// Lists commits on `origin/<main_branch>` that are new to the user.
///
/// News is every commit on `origin/<main>` that the user does not have yet and has not seen yet.
/// Commits reachable from `HEAD` or from the local `<main>` branch count as had (after Update, "Update main"
/// or a pull). Commits reachable from `last_seen` count as seen, when that commit exists in the repo
/// (it may be gone after a force-push).
/// Merge commits are included like any other commit.
/// An empty range gives an empty feed, not an error.
pub fn feed(
    repo: &Path,
    main_branch: &str,
    last_seen: Option<&str>,
    limit: usize,
) -> Result<Feed, GitError> {
    let run = |args: &[&str]| crate::git::run_background(repo, args);
    let mut revs = vec![format!("origin/{main_branch}"), "^HEAD".to_string()];
    let local_main = format!("refs/heads/{main_branch}");
    if run(&["rev-parse", "--verify", "--quiet", &local_main]).is_ok() {
        revs.push(format!("^{local_main}"));
    }
    if let Some(sha) = last_seen {
        if run(&["cat-file", "-e", &format!("{sha}^{{commit}}")]).is_ok() {
            revs.push(format!("^{sha}"));
        }
    }
    let max = format!("--max-count={limit}");
    let revs: Vec<&str> = revs.iter().map(String::as_str).collect();
    let git_log = |opts: &[&str]| run(&[opts, &["--end-of-options"], &revs[..]].concat());
    let log = git_log(&["log", "--format=%H%x1f%an%x1f%ae%x1f%at%x1f%s", &max])?;
    let total = git_log(&["rev-list", "--count"])?
        .trim()
        .parse()
        .unwrap_or(0);
    let names = git_log(&["log", "--name-only", "--format="])?;
    let mut files_changed: Vec<String> = names
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();
    files_changed.sort();
    files_changed.dedup();

    let me = run(&["config", "user.email"])
        .map(|e| e.trim().to_lowercase())
        .unwrap_or_default();
    let changes = log
        .lines()
        .filter_map(|line| {
            let mut f = line.splitn(5, '\x1f');
            let (sha, name, email, time, title) =
                (f.next()?, f.next()?, f.next()?, f.next()?, f.next()?);
            Some(Change {
                sha: sha.into(),
                author_name: name.into(),
                author_email: email.into(),
                title: title.into(),
                time: time.parse().ok()?,
                mine: !me.is_empty() && email.to_lowercase() == me,
            })
        })
        .collect();
    Ok(Feed {
        changes,
        total,
        files_changed,
    })
}

/// The commit `origin/<main_branch>` points to. Saved as `last_seen` when the user marks a repo as seen.
pub fn remote_main_sha(repo: &Path, main_branch: &str) -> Result<String, GitError> {
    let r = format!("refs/remotes/origin/{main_branch}^{{commit}}");
    let out = crate::git::run_background(repo, &["rev-parse", "--verify", "--quiet", &r])?;
    Ok(out.trim().to_string())
}
