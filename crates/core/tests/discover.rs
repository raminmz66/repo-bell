use std::fs;
use std::path::{Path, PathBuf};

use repobell_core::discover::{discover, SKIP_DIRS};
use tempfile::TempDir;

mod common;

/// Creates `dir/.git` as a folder, so `dir` is a repo.
fn make_repo(dir: &Path) {
    fs::create_dir_all(dir.join(".git")).expect("create .git");
}

fn make_dir(dir: &Path) {
    fs::create_dir_all(dir).expect("create dir");
}

fn root_of(tmp: &TempDir) -> PathBuf {
    tmp.path().join("root")
}

#[test]
fn finds_repos_at_depth_1_and_2_sorted() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_repo(&root.join("z-depth1"));
    make_repo(&root.join("b/c-depth2"));

    let found = discover(std::slice::from_ref(&root), 2);

    assert_eq!(found, vec![root.join("b/c-depth2"), root.join("z-depth1")]);
}

#[test]
fn repo_at_depth_3_is_not_found_with_max_depth_2() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_repo(&root.join("x/y/z"));

    assert!(discover(&[root], 2).is_empty());
}

#[test]
fn repo_at_depth_3_is_found_with_max_depth_3() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_repo(&root.join("x/y/z"));

    assert_eq!(discover(std::slice::from_ref(&root), 3), vec![root.join("x/y/z")]);
}

#[test]
fn root_that_is_a_repo_is_returned() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_repo(&root);

    assert_eq!(discover(std::slice::from_ref(&root), 2), vec![root]);
}

#[test]
fn repo_nested_inside_another_repo_is_not_returned() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_repo(&root.join("outer"));
    make_repo(&root.join("outer/inner"));

    assert_eq!(discover(std::slice::from_ref(&root), 3), vec![root.join("outer")]);
}

#[test]
fn hidden_folders_are_not_searched() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_repo(&root.join(".hidden/proj"));

    assert!(discover(&[root], 3).is_empty());
}

#[test]
fn node_modules_folder_is_not_searched() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_repo(&root.join("node_modules/proj"));

    assert!(discover(&[root], 3).is_empty());
}

#[test]
fn target_folder_is_not_searched() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_repo(&root.join("target/proj"));

    assert!(discover(&[root], 3).is_empty());
}

#[test]
fn every_skip_dirs_entry_is_not_searched() {
    for name in SKIP_DIRS {
        let tmp = TempDir::new().unwrap();
        let root = root_of(&tmp);
        make_repo(&root.join(name).join("proj"));

        assert!(
            discover(&[root], 3).is_empty(),
            "repo inside SKIP_DIRS entry {name:?} was found"
        );
    }
}

#[test]
fn dot_git_file_counts_as_repo() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    let worktree = root.join("worktree");
    make_dir(&worktree);
    fs::write(worktree.join(".git"), "gitdir: /somewhere\n").unwrap();

    assert_eq!(discover(std::slice::from_ref(&root), 2), vec![worktree]);
}

#[test]
fn missing_root_is_skipped_without_panic() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_repo(&root.join("real"));

    let found = discover(&[tmp.path().join("does-not-exist"), root.clone()], 2);

    assert_eq!(found, vec![root.join("real")]);
}

#[test]
fn overlapping_roots_give_no_duplicates() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_repo(&root.join("a"));

    let found = discover(&[root.clone(), root.join("a")], 2);

    assert_eq!(found, vec![root.join("a")]);
}

#[test]
fn symlinked_folder_pointing_at_repo_is_not_followed() {
    let tmp = TempDir::new().unwrap();
    let root = root_of(&tmp);
    make_dir(&root);
    make_repo(&tmp.path().join("outside/proj"));
    std::os::unix::fs::symlink(tmp.path().join("outside/proj"), root.join("link")).unwrap();

    assert!(discover(&[root], 2).is_empty());
}
