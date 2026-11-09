use std::path::Path;

use crate::git::GitError;

/// The state of one repo, read without touching the network.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoStatus {
    /// The checked-out branch, or `None` when HEAD is detached.
    pub branch: Option<String>,
    /// The remote's main branch name without the remote prefix ("main" or "master"),
    /// taken from `refs/remotes/origin/HEAD`, else `origin/main`, else `origin/master`.
    /// `None` when there is no `origin` or none of these exist.
    pub main_branch: Option<String>,
    /// True when the current branch is the main branch and tracks `origin/<main>`.
    /// False on a side branch, with no upstream, or when main is unknown.
    pub on_main: bool,
    /// The upstream of the current branch, such as "origin/main", or `None` when it has none.
    pub upstream: Option<String>,
    /// Commits on the upstream that the current branch does not have. 0 without an upstream.
    pub behind: u32,
    /// Commits on the current branch that the upstream does not have. 0 without an upstream.
    pub ahead: u32,
    /// True when there are staged, unstaged or untracked changes.
    pub unsaved: bool,
    /// Commits on `origin/<main>` that the local main branch does not have. Only counted when
    /// the checked-out branch is not main (0 otherwise, or when either branch is missing).
    pub main_behind: u32,
}

/// Reads the repo's status from local data only (call `fetch` first for fresh numbers).
/// Uses `git status --porcelain=v2 --branch`.
pub fn status(repo: &Path) -> Result<RepoStatus, GitError> {
    let out = crate::git::run_background(repo, &["status", "--porcelain=v2", "--branch"])?;
    let mut st = RepoStatus {
        branch: None,
        main_branch: None,
        on_main: false,
        upstream: None,
        behind: 0,
        ahead: 0,
        unsaved: false,
        main_behind: 0,
    };
    for line in out.lines() {
        if let Some(v) = line.strip_prefix("# branch.head ") {
            st.branch = (v != "(detached)").then(|| v.to_string());
        } else if let Some(v) = line.strip_prefix("# branch.upstream ") {
            st.upstream = Some(v.to_string());
        } else if let Some(v) = line.strip_prefix("# branch.ab ") {
            let mut n = v.split_whitespace().map(|x| x[1..].parse().unwrap_or(0));
            st.ahead = n.next().unwrap_or(0);
            st.behind = n.next().unwrap_or(0);
        } else if !line.starts_with('#') && !line.is_empty() {
            st.unsaved = true;
        }
    }
    st.main_branch = main_branch(repo);
    st.on_main = match (&st.branch, &st.main_branch, &st.upstream) {
        (Some(b), Some(m), Some(u)) => b == m && *u == format!("origin/{m}"),
        _ => false,
    };
    if let (Some(m), false) = (&st.main_branch, st.on_main) {
        let range = format!("refs/heads/{m}..refs/remotes/origin/{m}");
        st.main_behind =
            crate::git::run_background(repo, &["rev-list", "--count", "--end-of-options", &range])
                .ok()
                .and_then(|n| n.trim().parse().ok())
                .unwrap_or(0);
    }
    Ok(st)
}

pub(crate) fn main_branch(repo: &Path) -> Option<String> {
    let run = |args: &[&str]| crate::git::run_background(repo, args);
    if let Ok(s) = run(&[
        "symbolic-ref",
        "--quiet",
        "--short",
        "refs/remotes/origin/HEAD",
    ]) {
        let s = s.trim();
        return Some(s.strip_prefix("origin/").unwrap_or(s).to_string());
    }
    ["main", "master"].into_iter().find_map(|b| {
        run(&[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/remotes/origin/{b}"),
        ])
        .ok()
        .map(|_| b.to_string())
    })
}
