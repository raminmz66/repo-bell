use std::fmt;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// A failed Git call.
#[derive(Debug)]
pub enum GitError {
    /// The folder is missing or is not a Git repo.
    NotARepo,
    /// Git ran and exited with an error.
    Failed { args: String, stderr: String },
    /// Git could not be started at all.
    Io(std::io::Error),
    /// An update failed half-way and could not put the repo back. The user's unsaved edits are
    /// safe in the Git stash `stash` (a commit sha; `git stash apply <stash>` brings them back).
    /// `moved` is true when the branch had already moved to the team's version when it failed.
    EditsInStash {
        stash: String,
        reason: String,
        moved: bool,
    },
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotARepo => write!(f, "not a git repository"),
            GitError::Failed { args, stderr } => write!(f, "git {args} failed: {}", stderr.trim()),
            GitError::Io(e) => write!(f, "could not run git: {e}"),
            GitError::EditsInStash { stash, reason, .. } => write!(
                f,
                "your unsaved changes are safe in the Git stash {stash}: {}",
                reason.trim()
            ),
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
    let out = command(repo, args, envs, true)?
        .output()
        .map_err(GitError::Io)?;
    finish(args, out.status.success(), &out.stdout, &out.stderr)
}

/// Like `run`, for the read-only calls RepoBell makes on its own (status, log, rev-parse): the
/// repo's hooks are switched off. Update and Undo use `run`, so the user's hooks still run there.
pub(crate) fn run_background(repo: &Path, args: &[&str]) -> Result<String, GitError> {
    let out = command(repo, args, &[], false)?
        .output()
        .map_err(GitError::Io)?;
    finish(args, out.status.success(), &out.stdout, &out.stderr)
}

/// Like `run_with_env` for background network calls (fetch, ls-remote): repo hooks are switched
/// off, and Git (and everything it started) is killed after `timeout`.
/// A timeout is a `Failed` error whose stderr says "operation timed out".
pub(crate) fn run_with_timeout(
    repo: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
    timeout: Duration,
) -> Result<String, GitError> {
    let mut cmd = command(repo, args, envs, false)?;
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    // Own process group, so the whole tree (ssh, remote helpers) can be killed at once.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = cmd.spawn().map_err(GitError::Io)?;
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut buf);
            }
            let _ = tx.send(buf);
        });
        rx
    };
    let out = drain(child.stdout.take().map(|p| Box::new(p) as _));
    let err = drain(child.stderr.take().map(|p| Box::new(p) as _));
    let kill_group = |child: &mut std::process::Child| {
        #[cfg(unix)]
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .status();
        let _ = child.kill();
    };
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(st) = child.try_wait().map_err(GitError::Io)? {
            break st;
        }
        if Instant::now() >= deadline {
            kill_group(&mut child);
            let _ = child.wait();
            return Err(GitError::Failed {
                args: args.join(" "),
                stderr: format!("operation timed out: git did not finish within {timeout:?}"),
            });
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    // Git is done, but a helper it started may still hold the pipes open. Give the readers a
    // moment, then kill what is left of the group so they see the end of the output.
    let mut read = |rx: &mpsc::Receiver<Vec<u8>>| match rx.recv_timeout(Duration::from_millis(500))
    {
        Ok(buf) => buf,
        Err(_) => {
            kill_group(&mut child);
            rx.recv_timeout(Duration::from_secs(2)).unwrap_or_default()
        }
    };
    let (out, err) = (read(&out), read(&err));
    finish(args, status.success(), &out, &err)
}

fn command(
    repo: &Path,
    args: &[&str],
    envs: &[(&str, &str)],
    hooks: bool,
) -> Result<Command, GitError> {
    if !repo.is_dir() {
        return Err(GitError::NotARepo);
    }
    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(repo)
        // A repo dropped into a watched folder must not get to run a program via fsmonitor.
        .args(["-c", "core.quotePath=false", "-c", "core.fsmonitor=false"]);
    if !hooks {
        cmd.args(["-c", "core.hooksPath=/dev/null"]);
    }
    cmd.args(args)
        .env("LC_ALL", "C")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        // Empty (not unset) so Git ignores core.askPass and SSH_ASKPASS too.
        .env("GIT_ASKPASS", "")
        .env("SSH_ASKPASS", "")
        .env("SSH_ASKPASS_REQUIRE", "never")
        .env("GCM_INTERACTIVE", "never")
        .envs(envs.iter().copied());
    Ok(cmd)
}

fn finish(args: &[&str], ok: bool, stdout: &[u8], stderr: &[u8]) -> Result<String, GitError> {
    if ok {
        return Ok(String::from_utf8_lossy(stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(stderr).into_owned();
    if stderr.contains("not a git repository") {
        return Err(GitError::NotARepo);
    }
    Err(GitError::Failed {
        args: args.join(" "),
        stderr,
    })
}
