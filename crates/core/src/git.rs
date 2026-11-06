use std::fmt;
use std::path::Path;
use std::process::Command;

/// A failed Git call.
#[derive(Debug)]
pub enum GitError {
    /// The folder is missing or is not a Git repo.
    NotARepo,
    /// Git ran and exited with an error.
    Failed { args: String, stderr: String },
    /// Git could not be started at all.
    Io(std::io::Error),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotARepo => write!(f, "not a git repository"),
            GitError::Failed { args, stderr } => write!(f, "git {args} failed: {}", stderr.trim()),
            GitError::Io(e) => write!(f, "could not run git: {e}"),
        }
    }
}

impl std::error::Error for GitError {}

/// Runs `git <args>` inside `repo` and returns stdout.
/// Output is forced to English and Git never prompts for a password,
/// so a missing login fails fast instead of hanging.
/// `core.quotePath=false` keeps non-English file names as plain UTF-8, not octal escapes.
pub(crate) fn run(repo: &Path, args: &[&str]) -> Result<String, GitError> {
    run_with_env(repo, args, &[])
}

/// Like `run`, with extra environment variables set for this call.
pub(crate) fn run_with_env(
    repo: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<String, GitError> {
    if !repo.is_dir() {
        return Err(GitError::NotARepo);
    }
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["-c", "core.quotePath=false"])
        .args(args)
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .envs(envs.iter().copied())
        .output()
        .map_err(GitError::Io)?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
        if stderr.contains("not a git repository") {
            return Err(GitError::NotARepo);
        }
        Err(GitError::Failed {
            args: args.join(" "),
            stderr,
        })
    }
}
