use std::path::Path;
use std::time::Duration;

use crate::git::GitError;

/// Fetches only the remote's main branch from `origin`, updating `refs/remotes/origin/<main_branch>`.
///
/// Rules:
/// - Runs `git fetch --quiet --no-tags origin +refs/heads/<main>:refs/remotes/origin/<main>`.
/// - Never prompts: on top of the `GIT_TERMINAL_PROMPT=0` that every call already sets, SSH runs with
///   `BatchMode=yes` and `ConnectTimeout=15` (only when neither the `GIT_SSH_COMMAND` environment variable nor the repo's effective
///   `core.sshCommand` is set, since either one is the user's own SSH choice),
///   and slow HTTP transfers give up (`http.lowSpeedLimit=1000`, `http.lowSpeedTime=20`).
/// - Gives up after 60 seconds (also adds `ServerAliveInterval=15` and `ServerAliveCountMax=3` to the default SSH command).
/// - Does not touch the working tree, the index or any local branch.
/// - Returns `GitError::Failed` when the remote is missing or unreachable or the branch does not exist there.
pub fn fetch_main(repo: &Path, main_branch: &str) -> Result<(), GitError> {
    fetch_main_with_timeout(repo, main_branch, Duration::from_secs(60))
}

/// `fetch_main` with its own deadline. A fetch that takes longer is killed and fails with
/// "operation timed out" (which `classify` calls `Offline`).
///
/// When the remote has no branch of that name ("couldn't find remote ref", for example after the
/// default branch was renamed), asks the remote for its default branch, fetches that instead and
/// runs `git remote set-head origin --auto` so `status` follows the new name.
pub fn fetch_main_with_timeout(
    repo: &Path,
    main_branch: &str,
    timeout: Duration,
) -> Result<(), GitError> {
    let first = fetch_branch(repo, main_branch, timeout);
    let Err(GitError::Failed { stderr, .. }) = &first else {
        return first;
    };
    if !stderr.contains("couldn't find remote ref") {
        return first;
    }
    // Only a branch we believed to be main can have been renamed.
    if crate::status::main_branch(repo).as_deref() != Some(main_branch) {
        return first;
    }
    let remote_head = crate::git::run_with_timeout(
        repo,
        &["ls-remote", "--symref", "origin", "HEAD"],
        &ssh_env(repo),
        timeout,
    )
    .ok()
    .and_then(|out| {
        out.lines().find_map(|l| {
            l.strip_prefix("ref: refs/heads/")?
                .split('\t')
                .next()
                .map(str::to_string)
        })
    });
    match remote_head {
        Some(new) if new != main_branch => {
            fetch_branch(repo, &new, timeout)?;
            // Now that origin/<new> exists, point origin/HEAD at it so status finds the new main.
            let _ = crate::git::run_with_timeout(
                repo,
                &["remote", "set-head", "origin", "--auto"],
                &ssh_env(repo),
                timeout,
            );
            Ok(())
        }
        _ => first,
    }
}

pub(crate) fn ssh_env(repo: &Path) -> Vec<(&'static str, &'static str)> {
    let ssh = "ssh -o BatchMode=yes -o ConnectTimeout=15 -o ServerAliveInterval=15 -o ServerAliveCountMax=3";
    let user_ssh = std::env::var_os("GIT_SSH_COMMAND").is_some()
        || crate::git::run_background(repo, &["config", "--get", "core.sshCommand"])
            .is_ok_and(|v| !v.trim().is_empty());
    if user_ssh {
        vec![]
    } else {
        vec![("GIT_SSH_COMMAND", ssh)]
    }
}

fn fetch_branch(repo: &Path, main_branch: &str, timeout: Duration) -> Result<(), GitError> {
    let refspec = format!("+refs/heads/{main_branch}:refs/remotes/origin/{main_branch}");
    let args = [
        "-c",
        "http.lowSpeedLimit=1000",
        "-c",
        "http.lowSpeedTime=20",
        "fetch",
        "--quiet",
        "--no-tags",
        "origin",
        &refspec,
    ];
    crate::git::run_with_timeout(repo, &args, &ssh_env(repo), timeout).map(|_| ())
}
