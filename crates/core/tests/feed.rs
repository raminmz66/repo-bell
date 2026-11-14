use std::fs;
use std::path::Path;

use repobell_core::feed::{feed, remote_main_sha, Feed};
use repobell_core::GitError;
use tempfile::TempDir;

mod common;
use common::{commit_file, git, push_new_commits, setup_remote_and_clone};

/// Makes one commit per title in `dir` (each on its own new file) and pushes them.
fn push_titled(dir: &Path, titles: &[&str]) {
    for (i, title) in titles.iter().enumerate() {
        commit_file(dir, &format!("{title}-{i}.txt"), "x\n", title);
    }
    git(dir, &["push"]);
}

fn titles(feed: &Feed) -> Vec<&str> {
    feed.changes.iter().map(|c| c.title.as_str()).collect()
}

fn feed_of(work: &Path, last_seen: Option<&str>) -> Result<Feed, GitError> {
    feed(work, "main", last_seen, 50)
}

#[test]
fn feed_is_empty_when_no_new_commits() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    git(&work, &["fetch"]);

    let f = feed_of(&work, None).unwrap();

    assert!(f.changes.is_empty());
    assert_eq!(f.total, 0);
    assert!(f.files_changed.is_empty());
}

#[test]
fn feed_lists_pushed_commits_newest_first() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_titled(&other, &["first", "second", "third"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, None).unwrap();

    assert_eq!(titles(&f), ["third", "second", "first"]);
    assert_eq!(f.total, 3);
}

#[test]
fn feed_change_has_author_title_sha_and_time() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    git(&other, &["config", "user.name", "Sara Lee"]);
    git(&other, &["config", "user.email", "sara@example.com"]);
    fs::write(other.join("a.txt"), "a\n").unwrap();
    git(&other, &["add", "a.txt"]);
    git(&other, &["commit", "-m", "Title line", "-m", "Body text"]);
    git(&other, &["push"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, None).unwrap();
    let change = &f.changes[0];

    assert_eq!(change.author_name, "Sara Lee");
    assert_eq!(change.author_email, "sara@example.com");
    assert_eq!(change.title, "Title line");
    assert_eq!(change.sha.len(), 40);
    assert!(change.time > 0);
}

#[test]
fn feed_files_changed_is_sorted_and_unique() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    commit_file(&other, "zeta.txt", "1\n", "touch zeta");
    commit_file(&other, "alpha.txt", "1\n", "touch alpha");
    commit_file(&other, "zeta.txt", "2\n", "touch zeta again");
    git(&other, &["push"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, None).unwrap();

    assert_eq!(f.files_changed, ["alpha.txt", "zeta.txt"]);
}

#[test]
fn feed_limit_cuts_changes_but_keeps_total_and_files() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_titled(&other, &["c1", "c2", "c3", "c4", "c5"]);
    git(&work, &["fetch"]);

    let f = feed(&work, "main", None, 2).unwrap();

    assert_eq!(f.changes.len(), 2);
    assert_eq!(titles(&f), ["c5", "c4"]);
    assert_eq!(f.total, 5);
    assert_eq!(f.files_changed.len(), 5);
}

#[test]
fn feed_with_last_seen_lists_only_newer_commits() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_titled(&other, &["a1", "a2"]);
    git(&work, &["fetch"]);
    let seen = remote_main_sha(&work, "main").unwrap();
    push_titled(&other, &["d1", "d2", "d3"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, Some(&seen)).unwrap();

    assert_eq!(titles(&f), ["d3", "d2", "d1"]);
    assert_eq!(f.total, 3);
}

#[test]
fn feed_unknown_last_seen_falls_back_to_head_range() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_titled(&other, &["a1", "a2"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, Some("0123456789abcdef0123456789abcdef01234567")).unwrap();

    assert_eq!(f.total, 2);
}

#[test]
fn feed_without_last_seen_uses_head_range_after_pull() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_titled(&other, &["a1", "a2"]);
    git(&work, &["pull", "--ff-only"]);
    push_titled(&other, &["b1"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, None).unwrap();

    assert_eq!(titles(&f), ["b1"]);
    assert_eq!(f.total, 1);
}

#[test]
fn feed_marks_other_authors_as_not_mine() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    git(&other, &["config", "user.name", "Sara Lee"]);
    git(&other, &["config", "user.email", "sara@example.com"]);
    push_titled(&other, &["theirs"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, None).unwrap();

    assert!(!f.changes[0].mine);
}

#[test]
fn feed_marks_own_commit_as_mine() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    commit_file(&work, "mine.txt", "m\n", "mine");
    git(&work, &["push"]);
    git(&work, &["reset", "--hard", "HEAD~1"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, None).unwrap();

    assert_eq!(f.changes[0].title, "mine");
    assert!(f.changes[0].mine);
}

#[test]
fn feed_email_match_is_case_insensitive() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    commit_file(&work, "mine.txt", "m\n", "mine");
    git(&work, &["push"]);
    git(&work, &["reset", "--hard", "HEAD~1"]);
    git(&work, &["config", "user.email", "TEST@EXAMPLE.COM"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, None).unwrap();

    assert!(f.changes[0].mine);
}

#[test]
fn feed_mine_is_false_when_user_email_is_empty() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    commit_file(&work, "mine.txt", "m\n", "mine");
    git(&work, &["push"]);
    git(&work, &["reset", "--hard", "HEAD~1"]);
    git(&work, &["config", "user.email", ""]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, None).unwrap();

    assert_eq!(f.changes.len(), 1);
    assert!(!f.changes[0].mine);
}

#[test]
fn remote_main_sha_matches_origin_main() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_new_commits(&other, 2);
    git(&work, &["fetch"]);

    let sha = remote_main_sha(&work, "main").unwrap();

    assert_eq!(sha, git(&work, &["rev-parse", "origin/main"]).trim());
}

#[test]
fn feed_missing_branch_is_error() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());

    let result = feed(&work, "nope", None, 10);

    assert!(result.is_err(), "got {result:?}");
}

#[test]
fn feed_lists_non_english_file_names_unescaped() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    commit_file(&other, "سلام.txt", "x\n", "persian name");
    fs::create_dir_all(other.join("دکمه")).unwrap();
    commit_file(&other, "دکمه/رنگ.css", "x\n", "persian folder");
    git(&other, &["push"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, None).unwrap();

    // Git quotes non-ASCII paths by default ("\330\263..."); the name must come back as-is.
    assert!(
        f.files_changed.contains(&"سلام.txt".to_string()),
        "got {:?}",
        f.files_changed
    );
    assert!(
        f.files_changed.contains(&"دکمه/رنگ.css".to_string()),
        "got {:?}",
        f.files_changed
    );
}

#[test]
fn feed_pulled_changes_are_not_news() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    let seen = remote_main_sha(&work, "main").unwrap();
    push_titled(&other, &["a1", "a2"]);
    git(&work, &["pull", "--ff-only"]);

    let f = feed_of(&work, Some(&seen)).unwrap();

    assert_eq!(f.total, 0);
    assert!(f.changes.is_empty());
}

#[test]
fn feed_update_main_from_side_branch_is_not_news() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    let seen = remote_main_sha(&work, "main").unwrap();
    git(&work, &["switch", "-c", "side"]);
    push_titled(&other, &["a1", "a2"]);
    git(&work, &["fetch"]);
    git(&work, &["fetch", ".", "origin/main:main"]);

    let f = feed_of(&work, Some(&seen)).unwrap();

    assert_eq!(f.total, 0);
    assert!(f.changes.is_empty());
}

#[test]
fn feed_after_pull_only_counts_newer_pushes() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    let seen = remote_main_sha(&work, "main").unwrap();
    push_titled(&other, &["a1", "a2"]);
    git(&work, &["pull", "--ff-only"]);
    push_titled(&other, &["b1"]);
    git(&work, &["fetch"]);

    let f = feed_of(&work, Some(&seen)).unwrap();

    assert_eq!(titles(&f), ["b1"]);
    assert_eq!(f.total, 1);
}

#[test]
fn feed_merged_into_side_branch_is_not_news() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    let seen = remote_main_sha(&work, "main").unwrap();
    git(&work, &["switch", "-c", "side"]);
    push_titled(&other, &["a1", "a2"]);
    git(&work, &["fetch"]);
    git(&work, &["merge", "--no-edit", "origin/main"]);

    let f = feed_of(&work, Some(&seen)).unwrap();

    assert_eq!(f.total, 0);
    assert!(f.changes.is_empty());
}
