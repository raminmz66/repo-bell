use std::fs;

use repobell_core::status::{status, RepoStatus};
use repobell_core::GitError;
use tempfile::TempDir;

mod common;
use common::{
    commit_file, git, init_repo, push_new_commits, setup_remote_and_clone,
    setup_remote_and_clone_on,
};

fn clean_main_status() -> RepoStatus {
    RepoStatus {
        branch: Some("main".into()),
        main_branch: Some("main".into()),
        on_main: true,
        upstream: Some("origin/main".into()),
        behind: 0,
        ahead: 0,
        unsaved: false,
        main_behind: 0,
    }
}

#[test]
fn fresh_clone_reports_clean_main_tracking_origin() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());

    assert_eq!(status(&work).unwrap(), clean_main_status());
}

#[test]
fn behind_is_zero_until_fetch() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_new_commits(&other, 3);

    assert_eq!(status(&work).unwrap().behind, 0);
}

#[test]
fn behind_counts_remote_commits_after_fetch() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_new_commits(&other, 3);
    git(&work, &["fetch"]);

    let st = status(&work).unwrap();

    assert_eq!(st.behind, 3);
    assert_eq!(st.ahead, 0);
}

#[test]
fn ahead_counts_local_commits() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    commit_file(&work, "a.txt", "a\n", "local 1");
    commit_file(&work, "b.txt", "b\n", "local 2");

    let st = status(&work).unwrap();

    assert_eq!(st.ahead, 2);
    assert_eq!(st.behind, 0);
}

#[test]
fn behind_and_ahead_are_both_set() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_new_commits(&other, 1);
    commit_file(&work, "local.txt", "l\n", "local");
    git(&work, &["fetch"]);

    let st = status(&work).unwrap();

    assert_eq!(st.behind, 1);
    assert_eq!(st.ahead, 1);
}

#[test]
fn unsaved_is_true_for_modified_tracked_file() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    fs::write(work.join("README.md"), "changed\n").unwrap();

    assert!(status(&work).unwrap().unsaved);
}

#[test]
fn unsaved_is_true_for_staged_file() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    fs::write(work.join("staged.txt"), "s\n").unwrap();
    git(&work, &["add", "staged.txt"]);

    assert!(status(&work).unwrap().unsaved);
}

#[test]
fn unsaved_is_true_for_untracked_file() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    fs::write(work.join("new.txt"), "n\n").unwrap();

    assert!(status(&work).unwrap().unsaved);
}

#[test]
fn local_branch_without_upstream_has_no_upstream_and_zero_counts() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    git(&work, &["checkout", "-b", "feature"]);

    let st = status(&work).unwrap();

    assert_eq!(st.branch.as_deref(), Some("feature"));
    assert_eq!(st.upstream, None);
    assert_eq!(st.behind, 0);
    assert_eq!(st.ahead, 0);
}

#[test]
fn detached_head_has_no_branch() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    git(&work, &["checkout", "--detach"]);

    assert_eq!(status(&work).unwrap().branch, None);
}

#[test]
fn main_branch_falls_back_to_master_when_remote_default_is_master() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone_on(tmp.path(), "master");

    assert_eq!(
        status(&work).unwrap().main_branch.as_deref(),
        Some("master")
    );
}

#[test]
fn main_branch_is_none_without_remote() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("local");
    init_repo(&repo);
    commit_file(&repo, "a.txt", "a\n", "first");

    let st = status(&repo).unwrap();

    assert_eq!(st.main_branch, None);
    assert_eq!(st.upstream, None);
}

#[test]
fn nonexistent_path_is_not_a_repo() {
    let tmp = TempDir::new().unwrap();

    let result = status(&tmp.path().join("missing"));

    assert!(matches!(result, Err(GitError::NotARepo)), "got {result:?}");
}

#[test]
fn plain_folder_is_not_a_repo() {
    let tmp = TempDir::new().unwrap();
    let plain = tmp.path().join("plain");
    fs::create_dir_all(&plain).unwrap();
    fs::write(plain.join("file.txt"), "x\n").unwrap();

    let result = status(&plain);

    assert!(matches!(result, Err(GitError::NotARepo)), "got {result:?}");
}

#[test]
fn on_main_is_false_on_a_side_branch() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    assert!(status(&work).unwrap().on_main);

    git(&work, &["switch", "-c", "feature"]);
    let st = status(&work).unwrap();

    assert_eq!(st.branch.as_deref(), Some("feature"));
    assert_eq!(st.main_branch.as_deref(), Some("main"));
    assert!(!st.on_main);
}

#[test]
fn main_behind_counts_only_on_a_side_branch() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_new_commits(&other, 2);
    git(&work, &["fetch"]);
    assert_eq!(
        status(&work).unwrap().main_behind,
        0,
        "on main: Update covers it"
    );

    git(&work, &["checkout", "-b", "side"]);
    assert_eq!(status(&work).unwrap().main_behind, 2);

    git(&work, &["branch", "-f", "main", "origin/main"]);
    assert_eq!(status(&work).unwrap().main_behind, 0);
}
