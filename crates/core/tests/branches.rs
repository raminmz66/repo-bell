use std::path::{Path, PathBuf};

use repobell_core::{has_candidates, old_branches, server_branches};
use tempfile::TempDir;

mod common;
use common::{commit_file, git, init_repo, setup_remote_and_clone};

fn setup() -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    (tmp, work, other)
}

fn old(work: &Path) -> Vec<(String, bool)> {
    let server = server_branches(work).unwrap();
    old_branches(work, "main", &server)
        .unwrap()
        .into_iter()
        .map(|b| (b.name, b.merged))
        .collect()
}

/// Pushes branch `name` with one commit from `work` (tracking origin), then goes back to main.
fn push_branch(work: &Path, name: &str) {
    git(work, &["checkout", "-b", name]);
    commit_file(
        work,
        &format!("{}.txt", name.replace('/', "_")),
        "x\n",
        name,
    );
    git(work, &["push", "-u", "origin", name]);
    git(work, &["checkout", "main"]);
}

/// Merges `name` into main (fast-forward) and pushes.
fn merge_into_main(work: &Path, name: &str) {
    git(work, &["merge", "--ff-only", name]);
    git(work, &["push", "origin", "main"]);
}

fn delete_on_server(work: &Path, name: &str) {
    git(work, &["push", "origin", "--delete", name]);
    git(work, &["fetch"]);
}

fn gone_branch(work: &Path, name: &str) {
    push_branch(work, name);
    merge_into_main(work, name);
    delete_on_server(work, name);
}

#[test]
fn merged_branch_deleted_on_server_is_listed_as_merged() {
    let (_t, work, _o) = setup();
    gone_branch(&work, "feat");
    assert_eq!(old(&work), [("feat".to_string(), true)]);
}

#[test]
fn squash_merged_branch_is_listed_as_not_merged() {
    let (_t, work, other) = setup();
    push_branch(&work, "sq");
    git(&other, &["fetch"]);
    git(&other, &["merge", "--squash", "origin/sq"]);
    git(&other, &["commit", "-m", "squash sq"]);
    git(&other, &["push", "origin", "main"]);
    git(&work, &["push", "origin", "--delete", "sq"]);
    git(&work, &["fetch"]);
    assert_eq!(old(&work), [("sq".to_string(), false)]);
}

#[test]
fn branch_still_on_server_is_not_listed() {
    let (_t, work, _o) = setup();
    push_branch(&work, "develop");
    merge_into_main(&work, "develop");
    assert!(old(&work).is_empty());
}

#[test]
fn never_pushed_branch_is_not_listed() {
    let (_t, work, _o) = setup();
    git(&work, &["checkout", "-b", "local-only"]);
    commit_file(&work, "l.txt", "x\n", "local");
    git(&work, &["checkout", "main"]);
    assert!(old(&work).is_empty());
    assert!(!has_candidates(&work, "main").unwrap());
}

#[test]
fn branch_tracking_main_is_not_listed() {
    let (_t, work, _o) = setup();
    git(&work, &["checkout", "-b", "fresh", "origin/main"]);
    git(&work, &["checkout", "main"]);
    assert!(old(&work).is_empty());
    assert!(!has_candidates(&work, "main").unwrap());
}

#[test]
fn checked_out_branch_is_not_listed() {
    let (_t, work, _o) = setup();
    gone_branch(&work, "feat");
    git(&work, &["checkout", "feat"]);
    assert!(old(&work).is_empty());
}

#[test]
fn worktree_branch_is_not_listed() {
    let (_t, work, _o) = setup();
    gone_branch(&work, "feat");
    git(&work, &["worktree", "add", "../wt", "feat"]);
    assert!(old(&work).is_empty());
}

#[test]
fn slash_and_unicode_names_are_listed() {
    let (_t, work, _o) = setup();
    gone_branch(&work, "feature/login");
    gone_branch(&work, "fix/ünï");
    let mut names: Vec<String> = old(&work).into_iter().map(|b| b.0).collect();
    names.sort();
    assert_eq!(names, ["feature/login", "fix/ünï"]);
}

#[test]
fn no_origin_is_empty_not_error() {
    let tmp = TempDir::new().unwrap();
    init_repo(tmp.path());
    commit_file(tmp.path(), "a.txt", "x\n", "a");
    git(tmp.path(), &["branch", "b"]);
    assert!(!has_candidates(tmp.path(), "main").unwrap());
    assert!(server_branches(tmp.path()).is_err());
}

#[test]
fn detached_head_works() {
    let (_t, work, _o) = setup();
    gone_branch(&work, "feat");
    git(&work, &["checkout", "--detach"]);
    assert_eq!(old(&work), [("feat".to_string(), true)]);
}

#[test]
fn has_candidates_true_before_server_check() {
    let (_t, work, _o) = setup();
    push_branch(&work, "feat");
    assert!(has_candidates(&work, "main").unwrap());
    assert!(old(&work).is_empty());
}

#[test]
fn last_commit_time_is_the_tip_commit_time() {
    let (_t, work, _o) = setup();
    gone_branch(&work, "feat");
    let want: i64 = git(&work, &["log", "-1", "--format=%ct", "feat"])
        .trim()
        .parse()
        .unwrap();
    let server = server_branches(&work).unwrap();
    let got = old_branches(&work, "main", &server).unwrap();
    assert_eq!(got[0].last_commit_time, want);
}
