use std::fs;
use std::io;

use repobell_core::errors::{classify, Problem};
use repobell_core::fetch::fetch_main;
use repobell_core::GitError;
use tempfile::TempDir;

mod common;
use common::{git, setup_remote_and_clone};

fn failed(stderr: &str) -> GitError {
    GitError::Failed {
        args: "fetch".to_string(),
        stderr: stderr.to_string(),
    }
}

/// A folder that exists (so the NotARepo error is not a missing folder).
fn existing_dir(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().to_path_buf()
}

#[test]
fn login_failed_for_ssh_key_refused() {
    let tmp = TempDir::new().unwrap();
    let err = failed(
        "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.",
    );

    assert_eq!(classify(&err, &existing_dir(&tmp)), Problem::LoginFailed);
}

#[test]
fn login_failed_for_missing_username_prompt() {
    let tmp = TempDir::new().unwrap();
    let err = failed(
        "fatal: could not read Username for 'https://github.com': terminal prompts disabled",
    );

    assert_eq!(classify(&err, &existing_dir(&tmp)), Problem::LoginFailed);
}

#[test]
fn login_failed_for_http_403() {
    let tmp = TempDir::new().unwrap();
    let err = failed("fatal: unable to access 'https://github.com/x/y.git/': The requested URL returned error: 403");

    assert_eq!(classify(&err, &existing_dir(&tmp)), Problem::LoginFailed);
}

#[test]
fn offline_when_host_does_not_resolve() {
    let tmp = TempDir::new().unwrap();
    let err = failed(
        "fatal: unable to access 'https://github.com/x/y.git/': Could not resolve host: github.com",
    );

    assert_eq!(classify(&err, &existing_dir(&tmp)), Problem::Offline);
}

#[test]
fn offline_when_ssh_connection_times_out() {
    let tmp = TempDir::new().unwrap();
    let err = failed("ssh: connect to host github.com port 22: Connection timed out");

    assert_eq!(classify(&err, &existing_dir(&tmp)), Problem::Offline);
}

#[test]
fn remote_not_found_when_repository_is_missing() {
    let tmp = TempDir::new().unwrap();
    let err = failed(
        "remote: Repository not found.\nfatal: repository 'https://github.com/x/y.git/' not found",
    );

    assert_eq!(classify(&err, &existing_dir(&tmp)), Problem::RemoteNotFound);
}

#[test]
fn matching_is_case_insensitive() {
    let tmp = TempDir::new().unwrap();
    let err = failed("PERMISSION DENIED (PUBLICKEY).");

    assert_eq!(classify(&err, &existing_dir(&tmp)), Problem::LoginFailed);
}

#[test]
fn unknown_text_becomes_other_with_trimmed_stderr() {
    let tmp = TempDir::new().unwrap();
    let err = failed("  fatal: something odd happened\n");

    assert_eq!(
        classify(&err, &existing_dir(&tmp)),
        Problem::Other("fatal: something odd happened".to_string())
    );
}

#[test]
fn not_a_repo_with_missing_folder_is_folder_missing() {
    let tmp = TempDir::new().unwrap();
    let missing = tmp.path().join("gone");

    assert_eq!(
        classify(&GitError::NotARepo, &missing),
        Problem::FolderMissing
    );
}

#[test]
fn not_a_repo_with_existing_plain_folder_is_not_a_repo() {
    let tmp = TempDir::new().unwrap();
    let folder = tmp.path().join("plain");
    fs::create_dir_all(&folder).unwrap();

    assert_eq!(classify(&GitError::NotARepo, &folder), Problem::NotARepo);
}

#[test]
fn io_error_becomes_other_with_error_text() {
    let tmp = TempDir::new().unwrap();
    let err = GitError::Io(io::Error::other("boom"));

    match classify(&err, &existing_dir(&tmp)) {
        Problem::Other(text) => assert!(text.contains("boom"), "got {text:?}"),
        other => panic!("expected Other, got {other:?}"),
    }
}

#[test]
fn first_matching_group_wins() {
    let tmp = TempDir::new().unwrap();
    // Offline is checked before RemoteNotFound.
    let err = failed("could not resolve host: x\nrepository not found");

    assert_eq!(classify(&err, &existing_dir(&tmp)), Problem::Offline);
}

#[test]
fn fetch_from_missing_origin_is_remote_not_found() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    git(
        &work,
        &["remote", "set-url", "origin", "/nonexistent/path.git"],
    );

    let err = fetch_main(&work, "main").expect_err("fetch from missing origin must fail");

    assert_eq!(classify(&err, &work), Problem::RemoteNotFound);
}

#[test]
fn every_problem_has_a_message() {
    let problems = [
        Problem::Offline,
        Problem::LoginFailed,
        Problem::RemoteNotFound,
        Problem::FolderMissing,
        Problem::NotARepo,
        Problem::Other(String::new()),
    ];

    for problem in &problems {
        assert!(
            !problem.message().trim().is_empty(),
            "empty message for {problem:?}"
        );
    }
}
