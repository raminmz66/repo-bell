use std::path::Path;

use crate::git::GitError;

/// Fetches only the remote's main branch from `origin`, updating `refs/remotes/origin/<main_branch>`.
///
/// Rules:
/// - Runs `git fetch --quiet --no-tags origin +refs/heads/<main>:refs/remotes/origin/<main>`.
/// - Never prompts: on top of the `GIT_TERMINAL_PROMPT=0` that every call already sets, SSH runs with
///   `BatchMode=yes` and `ConnectTimeout=15` (only when neither the `GIT_SSH_COMMAND` environment variable nor the repo's effective
///   `core.sshCommand` is set, since either one is the user's own SSH choice),
///   and slow HTTP transfers give up (`http.lowSpeedLimit=1000`, `http.lowSpeedTime=20`).
/// - Does not touch the working tree, the index or any local branch.
/// - Returns `GitError::Failed` when the remote is missing or unreachable or the branch does not exist there.
pub fn fetch_main(repo: &Path, main_branch: &str) -> Result<(), GitError> {
    let refspec = format!("+refs/heads/{main_branch}:refs/remotes/origin/{main_branch}");
    let ssh = "ssh -o BatchMode=yes -o ConnectTimeout=15";
    let user_ssh = std::env::var_os("GIT_SSH_COMMAND").is_some()
        || crate::git::run(repo, &["config", "--get", "core.sshCommand"])
            .is_ok_and(|v| !v.trim().is_empty());
    let envs: &[(&str, &str)] = if user_ssh {
        &[]
    } else {
        &[("GIT_SSH_COMMAND", ssh)]
    };
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
    crate::git::run_with_env(repo, &args, envs).map(|_| ())
}
