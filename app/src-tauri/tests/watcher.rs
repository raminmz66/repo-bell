//! Runs the real check pipeline against throwaway Git repos.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use repobell_lib::store::{Seen, Settings};
use repobell_lib::watcher::{check_repositories, Cleanup, RepoLocks};
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "protocol.file.allow=always"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn clone(remote: &Path, dest: &Path, email: &str) {
    let out = Command::new("git")
        .args(["clone", "-q"])
        .arg(remote)
        .arg(dest)
        .output()
        .unwrap();
    assert!(out.status.success());
    git(dest, &["config", "user.email", email]);
    git(dest, &["config", "user.name", email]);
}

fn commit_and_push(dir: &Path, file: &str) {
    std::fs::write(dir.join(file), file).unwrap();
    git(dir, &["add", "."]);
    git(dir, &["commit", "-q", "-m", &format!("add {file}")]);
    git(dir, &["push", "-q", "origin", "HEAD"]);
}

#[test]
fn news_counts_teammate_changes_only_and_undo_point_follows_head() {
    let tmp = TempDir::new().unwrap();
    let remote = tmp.path().join("remote.git");
    git(
        tmp.path(),
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            remote.to_str().unwrap(),
        ],
    );
    let watched = tmp.path().join("watched");
    std::fs::create_dir(&watched).unwrap();

    let me = watched.join("app");
    clone(&remote, &me, "me@x.test");
    commit_and_push(&me, "first.txt");

    let bob = tmp.path().join("bob");
    clone(&remote, &bob, "bob@x.test");
    commit_and_push(&bob, "b1.txt");
    commit_and_push(&bob, "b2.txt");
    // My other computer: same email, so these are my own changes.
    let mine2 = tmp.path().join("mine2");
    clone(&remote, &mine2, "me@x.test");
    commit_and_push(&mine2, "m1.txt");

    let settings = Settings {
        folders: vec![watched.clone()],
        ..Settings::default()
    };
    let seen = Seen::new();
    let locks = RepoLocks::default();
    let (repos, all_failed) = check_repositories(
        &settings,
        &seen,
        &BTreeMap::new(),
        &mut Cleanup::default(),
        &locks,
    );
    assert!(!all_failed);
    let r = &repos[0];
    assert_eq!(r.error, None);
    assert_eq!(r.behind, 3);
    assert_eq!(r.new_count, 2, "own changes are not news");
    assert_eq!(r.feed.as_ref().unwrap().total_changes, 3);
    assert_eq!(r.teammate.len(), 2);
    assert!(r.on_main);
    assert_eq!(r.branch.as_deref(), Some("main"));

    // Seen = the remote tip: nothing is news any more, even though we're still behind.
    let tip = git(&me, &["rev-parse", "origin/main"]);
    let seen: Seen = [(me.clone(), tip)].into_iter().collect();
    let (repos, _) = check_repositories(
        &settings,
        &seen,
        &BTreeMap::new(),
        &mut Cleanup::default(),
        &locks,
    );
    assert_eq!(repos[0].new_count, 0);
    assert_eq!(repos[0].behind, 3);

    // Update, keep the undo point, and watch can_undo follow HEAD.
    let plan = repobell_core::plan(&me).unwrap();
    let repobell_core::Outcome::Updated { undo, .. } = repobell_core::update(&me, &plan).unwrap()
    else {
        panic!("expected an update");
    };
    let undo_map: BTreeMap<PathBuf, _> = [(me.clone(), undo)].into_iter().collect();
    let (repos, _) =
        check_repositories(&settings, &seen, &undo_map, &mut Cleanup::default(), &locks);
    assert!(repos[0].can_undo);
    std::fs::write(me.join("own.txt"), "x").unwrap();
    git(&me, &["add", "."]);
    git(&me, &["commit", "-q", "-m", "own"]);
    let (repos, _) =
        check_repositories(&settings, &seen, &undo_map, &mut Cleanup::default(), &locks);
    assert!(!repos[0].can_undo, "HEAD moved, so undo is gone");

    // A side branch is reported.
    git(&me, &["checkout", "-q", "-b", "side"]);
    let (repos, _) = check_repositories(
        &settings,
        &seen,
        &BTreeMap::new(),
        &mut Cleanup::default(),
        &locks,
    );
    assert!(!repos[0].on_main);
    assert_eq!(repos[0].branch.as_deref(), Some("side"));
}

fn repo_with_origin(watched: &Path, name: &str, url: &str) -> PathBuf {
    let repo = watched.join(name);
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    git(&repo, &["remote", "add", "origin", url]);
    repo
}

#[test]
fn a_deleted_remote_is_an_error_but_not_offline() {
    let tmp = TempDir::new().unwrap();
    let watched = tmp.path().join("w");
    let repo = repo_with_origin(&watched, "r", "/nonexistent/remote.git");
    let _ = repo;
    let settings = Settings {
        folders: vec![watched],
        ..Settings::default()
    };
    let (repos, offline) = check_repositories(
        &settings,
        &Seen::new(),
        &BTreeMap::new(),
        &mut Cleanup::default(),
        &RepoLocks::default(),
    );
    assert!(repos[0].error.is_some());
    assert_eq!(repos[0].error_kind.as_deref(), Some("not_found"));
    assert!(!offline, "one repo's own problem is not 'no internet'");
    assert_eq!(repos[0].new_count, 0, "errored repos are not news");
}

#[test]
fn offline_only_when_every_repo_can_not_reach_the_server() {
    use std::os::unix::fs::PermissionsExt;
    if std::env::var_os("GIT_SSH_COMMAND").is_some() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let watched = tmp.path().join("w");
    let ssh = tmp.path().join("ssh.sh");
    std::fs::write(
        &ssh,
        "#!/bin/sh\necho 'ssh: connect to host x port 22: Connection timed out' >&2\nexit 255\n",
    )
    .unwrap();
    std::fs::set_permissions(&ssh, std::fs::Permissions::from_mode(0o755)).unwrap();
    let a = repo_with_origin(&watched, "a", "ssh://nobody@example.invalid/a.git");
    git(&a, &["config", "core.sshCommand", ssh.to_str().unwrap()]);
    let settings = Settings {
        folders: vec![watched.clone()],
        ..Settings::default()
    };
    let check = || {
        check_repositories(
            &settings,
            &Seen::new(),
            &BTreeMap::new(),
            &mut Cleanup::default(),
            &RepoLocks::default(),
        )
    };
    let (repos, offline) = check();
    assert_eq!(repos[0].error_kind.as_deref(), Some("offline"));
    assert!(offline);

    // A second repo with a different problem: the check as a whole is not offline.
    repo_with_origin(&watched, "b", "/nonexistent/remote.git");
    let (_, offline) = check();
    assert!(!offline);
}

/// A remote, a watched folder and a clone of it with `feat` pushed, merged into main and pushed.
fn merged_feat() -> (TempDir, PathBuf, PathBuf, Settings) {
    let tmp = TempDir::new().unwrap();
    let remote = tmp.path().join("remote.git");
    git(
        tmp.path(),
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            remote.to_str().unwrap(),
        ],
    );
    let watched = tmp.path().join("watched");
    std::fs::create_dir(&watched).unwrap();
    let me = watched.join("app");
    clone(&remote, &me, "me@x.test");
    commit_and_push(&me, "first.txt");
    git(&me, &["checkout", "-q", "-b", "feat"]);
    std::fs::write(me.join("f.txt"), "f").unwrap();
    git(&me, &["add", "."]);
    git(&me, &["commit", "-q", "-m", "feat"]);
    git(&me, &["push", "-q", "-u", "origin", "feat"]);
    git(&me, &["checkout", "-q", "main"]);
    git(&me, &["merge", "-q", "feat"]);
    git(&me, &["push", "-q", "origin", "main"]);
    let settings = Settings {
        folders: vec![watched],
        ..Settings::default()
    };
    (tmp, remote, me, settings)
}

fn check(settings: &Settings, cleanup: &mut Cleanup) -> repobell_lib::watcher::RepoCardView {
    let (mut repos, _) = check_repositories(
        settings,
        &Seen::new(),
        &BTreeMap::new(),
        cleanup,
        &RepoLocks::default(),
    );
    repos.remove(0)
}

#[test]
fn card_lists_branch_deleted_on_server() {
    let (_tmp, remote, _me, settings) = merged_feat();
    let before = check(&settings, &mut Cleanup::default());
    assert!(before.old_branches.is_empty());
    git(&remote, &["branch", "-D", "feat"]);
    let after = check(&settings, &mut Cleanup::default());
    assert_eq!(after.old_branches.len(), 1);
    assert_eq!(after.old_branches[0].name, "feat");
    assert!(after.old_branches[0].merged);
    assert_eq!(after.new_count, before.new_count);
    assert_eq!(after.error, before.error);
}

#[test]
fn server_answer_is_cached_for_an_hour() {
    let (_tmp, remote, me, settings) = merged_feat();
    git(&me, &["branch", "feat2", "feat"]);
    git(&me, &["push", "-q", "-u", "origin", "feat2"]);
    git(&remote, &["branch", "-D", "feat"]);
    let mut cleanup = Cleanup::default();
    assert_eq!(check(&settings, &mut cleanup).old_branches.len(), 1);
    git(&remote, &["branch", "-D", "feat2"]);
    assert_eq!(check(&settings, &mut cleanup).old_branches.len(), 1);
    for e in cleanup.server.values_mut() {
        e.checked_at -= 3601;
    }
    assert_eq!(check(&settings, &mut cleanup).old_branches.len(), 2);
}

#[test]
fn no_line_when_server_never_answered() {
    let (_tmp, _remote, me, settings) = merged_feat();
    git(
        &me,
        &["remote", "set-url", "origin", "/nonexistent/remote.git"],
    );
    let mut cleanup = Cleanup::default();
    let card = check(&settings, &mut cleanup);
    assert!(card.old_branches.is_empty());
    assert!(cleanup.server.is_empty());
    // Only the fetch error: same kind and message as a plain repo with the same broken remote.
    let plain = TempDir::new().unwrap();
    repo_with_origin(plain.path(), "p", "/nonexistent/remote.git");
    let plain_settings = Settings {
        folders: vec![plain.path().to_path_buf()],
        ..Settings::default()
    };
    let plain_card = check(&plain_settings, &mut Cleanup::default());
    assert_eq!(card.error_kind.as_deref(), Some("not_found"));
    assert_eq!(card.error, plain_card.error);
}

#[test]
fn no_server_call_after_a_failed_fetch() {
    let (_tmp, remote, me, settings) = merged_feat();
    git(&remote, &["branch", "-D", "feat"]);
    // ls-remote still works, but fetching main fails: its ref in the clone is locked.
    let bob = _tmp.path().join("bob");
    clone(&remote, &bob, "bob@x.test");
    commit_and_push(&bob, "b1.txt");
    std::fs::write(me.join(".git/refs/remotes/origin/main.lock"), "").unwrap();
    let mut cleanup = Cleanup::default();
    let card = check(&settings, &mut cleanup);
    assert!(card.error.is_some(), "fetching main must fail here");
    assert!(
        cleanup.server.is_empty(),
        "no ls-remote after a failed fetch"
    );
}

#[test]
fn old_list_kept_when_server_fails_later() {
    let (_tmp, remote, me, settings) = merged_feat();
    git(&remote, &["branch", "-D", "feat"]);
    let mut cleanup = Cleanup::default();
    assert_eq!(check(&settings, &mut cleanup).old_branches.len(), 1);
    for e in cleanup.server.values_mut() {
        e.checked_at -= 3601;
    }
    git(
        &me,
        &["remote", "set-url", "origin", "/nonexistent/remote.git"],
    );
    assert_eq!(check(&settings, &mut cleanup).old_branches.len(), 1);
}

#[test]
fn no_server_call_without_candidates() {
    let tmp = TempDir::new().unwrap();
    let remote = tmp.path().join("remote.git");
    git(
        tmp.path(),
        &[
            "init",
            "-q",
            "--bare",
            "-b",
            "main",
            remote.to_str().unwrap(),
        ],
    );
    let watched = tmp.path().join("watched");
    std::fs::create_dir(&watched).unwrap();
    let me = watched.join("app");
    clone(&remote, &me, "me@x.test");
    commit_and_push(&me, "first.txt");
    git(&me, &["branch", "local-only"]);
    let settings = Settings {
        folders: vec![watched],
        ..Settings::default()
    };
    let mut cleanup = Cleanup::default();
    check(&settings, &mut cleanup);
    assert!(cleanup.server.is_empty());
}

#[test]
fn candidate_missing_from_the_answer_triggers_a_refresh() {
    let (_tmp, remote, me, settings) = merged_feat();
    let mut cleanup = Cleanup::default();
    assert!(check(&settings, &mut cleanup).old_branches.is_empty());
    git(&me, &["branch", "feat2", "feat"]);
    git(&me, &["push", "-q", "-u", "origin", "feat2"]);
    git(&remote, &["branch", "-D", "feat"]);
    // The saved answer is fresh, but it never covered feat2, so the server is asked again.
    let card = check(&settings, &mut cleanup);
    let names: Vec<_> = card.old_branches.iter().map(|b| b.name.as_str()).collect();
    assert_eq!(names, ["feat"]);
}

#[test]
fn second_delete_adds_to_the_undo_list_newest_first() {
    let d = |n: &str| repobell_core::DeletedBranch {
        name: n.into(),
        sha: "1".repeat(40),
        remote: String::new(),
        merge: String::new(),
    };
    let mut cleanup = Cleanup::default();
    let path = std::path::PathBuf::from("/r");
    cleanup.add_undo(path.clone(), vec![d("a")]);
    cleanup.add_undo(path.clone(), vec![d("b"), d("c")]);
    let names: Vec<_> = cleanup.undo[&path]
        .iter()
        .map(|x| x.name.as_str())
        .collect();
    assert_eq!(names, ["b", "c", "a"]);
}
