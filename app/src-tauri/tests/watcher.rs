//! Runs the real check pipeline against throwaway Git repos.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use repobell_lib::store::{Seen, Settings};
use repobell_lib::watcher::{check_repositories, RepoLocks};
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
    let (repos, all_failed) = check_repositories(&settings, &seen, &BTreeMap::new(), &locks);
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
    let (repos, _) = check_repositories(&settings, &seen, &BTreeMap::new(), &locks);
    assert_eq!(repos[0].new_count, 0);
    assert_eq!(repos[0].behind, 3);

    // Update, keep the undo point, and watch can_undo follow HEAD.
    let plan = repobell_core::plan(&me).unwrap();
    let repobell_core::Outcome::Updated { undo, .. } = repobell_core::update(&me, &plan).unwrap()
    else {
        panic!("expected an update");
    };
    let undo_map: BTreeMap<PathBuf, _> = [(me.clone(), undo)].into_iter().collect();
    let (repos, _) = check_repositories(&settings, &seen, &undo_map, &locks);
    assert!(repos[0].can_undo);
    std::fs::write(me.join("own.txt"), "x").unwrap();
    git(&me, &["add", "."]);
    git(&me, &["commit", "-q", "-m", "own"]);
    let (repos, _) = check_repositories(&settings, &seen, &undo_map, &locks);
    assert!(!repos[0].can_undo, "HEAD moved, so undo is gone");

    // A side branch is reported.
    git(&me, &["checkout", "-q", "-b", "side"]);
    let (repos, _) = check_repositories(&settings, &seen, &BTreeMap::new(), &locks);
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
