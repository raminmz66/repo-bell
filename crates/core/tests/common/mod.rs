//! Helpers for integration tests. They drive the real `git` CLI.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Runs `git -C <dir> <args>` with English output and returns stdout. Panics with stderr on failure.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .expect("failed to start git");
    assert!(
        out.status.success(),
        "git {:?} failed in {}: {}",
        args,
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn set_identity(dir: &Path) {
    git(dir, &["config", "user.name", "Test"]);
    git(dir, &["config", "user.email", "test@example.com"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
}

fn init_repo_on(dir: &Path, branch: &str) {
    fs::create_dir_all(dir).expect("create repo dir");
    git(dir, &["init", "-b", branch]);
    set_identity(dir);
}

/// Creates a new repo in `dir` on branch `main`, with user and gpg settings.
pub fn init_repo(dir: &Path) {
    init_repo_on(dir, "main");
}

/// Writes `name` in `dir`, stages it and commits it with `msg`.
pub fn commit_file(dir: &Path, name: &str, content: &str, msg: &str) {
    fs::write(dir.join(name), content).expect("write file");
    git(dir, &["add", name]);
    git(dir, &["commit", "-m", msg]);
}

/// Creates a bare `remote.git` with default branch `branch`, seeds it with one commit,
/// then clones it into `work` and `other` under `tmp`. Returns (remote, work, other).
pub fn setup_remote_and_clone_on(tmp: &Path, branch: &str) -> (PathBuf, PathBuf, PathBuf) {
    let remote = tmp.join("remote.git");
    git(tmp, &["init", "--bare", "-b", branch, "remote.git"]);
    let remote_str = remote.to_str().expect("utf-8 path");

    let seed = tmp.join("seed");
    init_repo_on(&seed, branch);
    commit_file(&seed, "README.md", "hello\n", "initial commit");
    git(&seed, &["remote", "add", "origin", remote_str]);
    git(&seed, &["push", "-u", "origin", branch]);

    let work = tmp.join("work");
    let other = tmp.join("other");
    git(tmp, &["clone", remote_str, "work"]);
    git(tmp, &["clone", remote_str, "other"]);
    set_identity(&work);
    set_identity(&other);

    (remote, work, other)
}

/// Same as `setup_remote_and_clone_on` with branch `main`.
pub fn setup_remote_and_clone(tmp: &Path) -> (PathBuf, PathBuf, PathBuf) {
    setup_remote_and_clone_on(tmp, "main")
}

/// Makes `n` commits in `other` (one new file each) and pushes them.
pub fn push_new_commits(other: &Path, n: usize) {
    for _ in 0..n {
        let count = git(other, &["rev-list", "--count", "HEAD"])
            .trim()
            .to_string();
        commit_file(
            other,
            &format!("commit-{count}.txt"),
            "x\n",
            &format!("commit {count}"),
        );
    }
    git(other, &["push"]);
}

/// Runs `git -C <dir> <args>`, expects it to fail, and returns stderr. Panics if it succeeds.
pub fn git_expect_fail(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .expect("failed to start git");
    assert!(
        !out.status.success(),
        "git {:?} unexpectedly succeeded in {}",
        args,
        dir.display()
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Everything the user could lose: HEAD sha, `git status --porcelain` (untracked files included)
/// and the bytes of every file in the working tree (`.git` excluded), keyed by relative path.
pub type Snapshot = (String, String, BTreeMap<PathBuf, Vec<u8>>);

pub fn snapshot(repo: &Path) -> Snapshot {
    let head = git(repo, &["rev-parse", "HEAD"]).trim().to_string();
    let status = git(repo, &["status", "--porcelain", "--untracked-files=all"]);
    let mut files = BTreeMap::new();
    collect_files(repo, repo, &mut files);
    (head, status, files)
}

fn collect_files(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let entry = entry.expect("dir entry");
        if entry.file_name() == ".git" {
            continue;
        }
        let path = entry.path();
        let kind = entry.file_type().expect("file type");
        if kind.is_dir() {
            collect_files(root, &path, out);
        } else if kind.is_file() {
            let rel = path.strip_prefix(root).expect("under root").to_path_buf();
            out.insert(rel, fs::read(&path).expect("read file"));
        }
    }
}

/// Panics unless `git stash list` is empty.
pub fn assert_no_stash(repo: &Path) {
    let list = git(repo, &["stash", "list"]);
    assert!(list.is_empty(), "stash list not empty: {list}");
}
