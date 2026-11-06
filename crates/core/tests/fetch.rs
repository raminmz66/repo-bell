use std::fs;

use repobell_core::fetch::fetch_main;
use repobell_core::GitError;
use tempfile::TempDir;

mod common;
use common::{commit_file, git, init_repo, push_new_commits, setup_remote_and_clone};

#[test]
fn fetch_main_updates_origin_main_to_remote_head() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_new_commits(&other, 2);

    fetch_main(&work, "main").unwrap();

    assert_eq!(
        git(&work, &["rev-parse", "origin/main"]).trim(),
        git(&other, &["rev-parse", "HEAD"]).trim()
    );
}

#[test]
fn fetch_main_keeps_head_local_main_and_working_tree() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_new_commits(&other, 2);
    let head_before = git(&work, &["rev-parse", "HEAD"]);
    let main_before = git(&work, &["rev-parse", "main"]);
    fs::write(work.join("README.md"), "unsaved edit\n").unwrap();

    fetch_main(&work, "main").unwrap();

    assert_eq!(git(&work, &["rev-parse", "HEAD"]), head_before);
    assert_eq!(git(&work, &["rev-parse", "main"]), main_before);
    assert_eq!(
        fs::read_to_string(work.join("README.md")).unwrap(),
        "unsaved edit\n"
    );
}

#[test]
fn fetch_main_missing_branch_is_failed() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());

    let result = fetch_main(&work, "nope");

    assert!(
        matches!(result, Err(GitError::Failed { .. })),
        "got {result:?}"
    );
}

#[test]
fn fetch_main_unreachable_origin_is_failed() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    git(
        &work,
        &["remote", "set-url", "origin", "/nonexistent/path.git"],
    );

    let result = fetch_main(&work, "main");

    assert!(
        matches!(result, Err(GitError::Failed { .. })),
        "got {result:?}"
    );
}

#[test]
fn fetch_main_without_remote_is_error() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("local");
    init_repo(&repo);
    commit_file(&repo, "a.txt", "a\n", "first");

    let result = fetch_main(&repo, "main");

    assert!(result.is_err(), "got {result:?}");
}

#[cfg(unix)]
#[test]
fn fetch_main_respects_repo_core_ssh_command() {
    // If the test process already has GIT_SSH_COMMAND, Git uses that instead of the
    // repo's core.sshCommand, so the result says nothing about the repo setting.
    if std::env::var_os("GIT_SSH_COMMAND").is_some() {
        return;
    }
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    init_repo(&repo);
    let marker = tmp.path().join("marker");
    let script = tmp.path().join("fake-ssh.sh");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch {}\nexit 1\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    git(
        &repo,
        &["config", "core.sshCommand", script.to_str().unwrap()],
    );
    git(
        &repo,
        &["remote", "add", "origin", "ssh://example.invalid/repo.git"],
    );

    let result = fetch_main(&repo, "main");

    assert!(result.is_err(), "got {result:?}");
    assert!(
        marker.exists(),
        "fake ssh was not run; RepoBell overrode core.sshCommand"
    );
}
