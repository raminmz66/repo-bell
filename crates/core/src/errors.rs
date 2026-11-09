use std::path::Path;

use crate::git::GitError;

/// A failure in plain words, for the repo card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The server can't be reached.
    Offline,
    /// Git could not log in (SSH key refused, token expired, unknown host key...).
    LoginFailed,
    /// The server answered but the repo is not there (renamed, deleted, or no access).
    RemoteNotFound,
    /// The repo folder is gone.
    FolderMissing,
    /// The folder exists but is no longer a Git repo.
    NotARepo,
    /// Anything else. Keeps Git's own message for a details view.
    Other(String),
}

impl Problem {
    /// The sentence shown on the card.
    pub fn message(&self) -> &'static str {
        match self {
            Problem::Offline => "Can't reach the server. Check your internet connection.",
            Problem::LoginFailed => "Git can't log in to the server. Your login may have expired.",
            Problem::RemoteNotFound => {
                "The repo was not found on the server. It may have been renamed or deleted."
            }
            Problem::FolderMissing => "Folder not found. It may have been moved or deleted.",
            Problem::NotARepo => "This folder is no longer a Git repo.",
            Problem::Other(_) => "Something went wrong while checking this repo.",
        }
    }
}

/// Sorts a Git failure for `repo` into a `Problem`, by matching Git's English error text
/// (every call runs with `LC_ALL=C`).
///
/// - `NotARepo` error: `FolderMissing` when `repo` does not exist, else `NotARepo`.
/// - `Io` and `EditsInStash` errors: `Other` with the error text.
/// - `Failed` error, matched on stderr, case-insensitive, first match wins in this order:
///   LoginFailed: "permission denied (publickey", "authentication failed", "could not read username",
///   "terminal prompts disabled", "host key verification failed", "the requested url returned error: 403"
///   Offline: "could not resolve host", "network is unreachable", "connection timed out",
///   "operation timed out", "connection refused", "failed to connect", "no route to host"
///   RemoteNotFound: "repository not found", "does not appear to be a git repository",
///   "the requested url returned error: 404"
///   else `Other(stderr trimmed)`.
pub fn classify(err: &GitError, repo: &Path) -> Problem {
    let stderr = match err {
        GitError::NotARepo if repo.exists() => return Problem::NotARepo,
        GitError::NotARepo => return Problem::FolderMissing,
        GitError::Io(e) => return Problem::Other(e.to_string()),
        GitError::EditsInStash { .. } => return Problem::Other(err.to_string()),
        GitError::Failed { stderr, .. } => stderr,
    };
    let low = stderr.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| low.contains(n));
    if has(&[
        "permission denied (publickey",
        "authentication failed",
        "could not read username",
        "terminal prompts disabled",
        "host key verification failed",
        "the requested url returned error: 403",
    ]) {
        Problem::LoginFailed
    } else if has(&[
        "could not resolve host",
        "network is unreachable",
        "connection timed out",
        "operation timed out",
        "connection refused",
        "failed to connect",
        "no route to host",
    ]) {
        Problem::Offline
    } else if has(&[
        "repository not found",
        "does not appear to be a git repository",
        "the requested url returned error: 404",
    ]) {
        Problem::RemoteNotFound
    } else {
        Problem::Other(stderr.trim().to_string())
    }
}
