use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use repobell_core::update::{can_undo, plan, undo, update, Blocked, Outcome, Plan, UndoOutcome};
use tempfile::TempDir;

mod common;
use common::{
    assert_no_stash, commit_file, git, git_expect_fail, push_new_commits, setup_remote_and_clone,
    snapshot,
};

/// Fresh remote with one commit: `work` is the user's clone, `other` is the team's clone.
fn setup() -> (TempDir, PathBuf, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    (tmp, work, other)
}

fn fetch(dir: &Path) {
    git(dir, &["fetch"]);
}

fn head(dir: &Path) -> String {
    git(dir, &["rev-parse", "HEAD"]).trim().to_string()
}

fn origin_main(dir: &Path) -> String {
    git(dir, &["rev-parse", "origin/main"]).trim().to_string()
}

/// True when `git <args>` exits with success. Does not panic on failure.
fn git_ok(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .expect("failed to start git")
        .status
        .success()
}

fn merge_in_progress(dir: &Path) -> bool {
    git_ok(dir, &["rev-parse", "-q", "--verify", "MERGE_HEAD"])
}

// ---- plan ----

#[test]
fn plan_up_to_date_when_nothing_to_download() {
    let (_tmp, work, _other) = setup();
    fetch(&work);

    assert_eq!(plan(&work).unwrap(), Plan::UpToDate);
}

#[test]
fn plan_fast_forward_when_behind_and_clean() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 2);
    fetch(&work);

    assert_eq!(
        plan(&work).unwrap(),
        Plan::FastForward {
            downloads: 2,
            set_aside: false
        }
    );
}

#[test]
fn plan_fast_forward_sets_aside_when_tracked_file_is_modified() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    fs::write(work.join("README.md"), "my unsaved edit\n").unwrap();
    fetch(&work);

    assert_eq!(
        plan(&work).unwrap(),
        Plan::FastForward {
            downloads: 1,
            set_aside: true
        }
    );
}

#[test]
fn plan_fast_forward_sets_aside_when_only_untracked_file_exists() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    fs::write(work.join("notes.txt"), "todo\n").unwrap();
    fetch(&work);

    assert_eq!(
        plan(&work).unwrap(),
        Plan::FastForward {
            downloads: 1,
            set_aside: true
        }
    );
}

#[test]
fn plan_combine_when_behind_and_ahead() {
    let (_tmp, work, other) = setup();
    commit_file(&work, "local.txt", "mine\n", "local change");
    push_new_commits(&other, 2);
    fetch(&work);

    assert_eq!(
        plan(&work).unwrap(),
        Plan::Combine {
            downloads: 2,
            yours: 1,
            set_aside: false
        }
    );
}

#[test]
fn plan_up_to_date_when_only_ahead() {
    let (_tmp, work, _other) = setup();
    commit_file(&work, "local.txt", "mine\n", "local change");
    fetch(&work);

    assert_eq!(plan(&work).unwrap(), Plan::UpToDate);
}

#[test]
fn plan_blocked_when_detached() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    git(&work, &["checkout", "--detach"]);
    fetch(&work);

    assert_eq!(plan(&work).unwrap(), Plan::Blocked(Blocked::Detached));
}

#[test]
fn plan_blocked_when_branch_has_no_upstream() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    git(&work, &["checkout", "-b", "feature"]);
    assert!(!git_ok(&work, &["rev-parse", "--abbrev-ref", "@{u}"]));
    fetch(&work);

    assert_eq!(plan(&work).unwrap(), Plan::Blocked(Blocked::NoUpstream));
}

#[test]
fn plan_blocked_when_merge_in_progress() {
    let (_tmp, work, other) = setup();
    commit_file(&work, "README.md", "mine\n", "my edit");
    commit_file(&other, "README.md", "theirs\n", "team edit");
    git(&other, &["push"]);
    fetch(&work);
    git_expect_fail(&work, &["merge", "origin/main"]);
    assert!(merge_in_progress(&work));

    assert_eq!(plan(&work).unwrap(), Plan::Blocked(Blocked::InProgress));
}

// ---- update ----

#[test]
fn update_fast_forward_moves_branch_and_reports_undo_point() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 2);
    fetch(&work);
    let old = head(&work);
    let expected = plan(&work).unwrap();

    let outcome = update(&work, &expected).unwrap();

    let Outcome::Updated { downloads, undo } = outcome else {
        panic!("expected Updated, got {outcome:?}");
    };
    assert_eq!(downloads, 2);
    assert_eq!(undo.before, old);
    assert_eq!(undo.after, head(&work));
    assert_eq!(head(&work), origin_main(&work));
    assert_no_stash(&work);
}

#[test]
fn update_set_aside_keeps_unsaved_edit_and_untracked_file() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    fs::write(work.join("README.md"), "my unsaved edit\n").unwrap();
    fs::write(work.join("notes.txt"), "todo\n").unwrap();
    fetch(&work);
    let expected = plan(&work).unwrap();

    let outcome = update(&work, &expected).unwrap();

    assert!(
        matches!(outcome, Outcome::Updated { .. }),
        "got {outcome:?}"
    );
    assert_eq!(head(&work), origin_main(&work));
    assert_eq!(
        fs::read_to_string(work.join("README.md")).unwrap(),
        "my unsaved edit\n"
    );
    assert_eq!(
        fs::read_to_string(work.join("notes.txt")).unwrap(),
        "todo\n"
    );
    assert_no_stash(&work);
}

#[test]
fn update_set_aside_pop_conflict_restores_exactly() {
    let (_tmp, work, other) = setup();
    commit_file(&other, "README.md", "hello from the team\n", "team edit");
    git(&other, &["push"]);
    fs::write(work.join("README.md"), "my unsaved edit\n").unwrap();
    fs::write(work.join("notes.txt"), "todo\n").unwrap();
    fetch(&work);
    let expected = plan(&work).unwrap();
    let before = snapshot(&work);

    let outcome = update(&work, &expected).unwrap();

    assert_eq!(
        outcome,
        Outcome::Conflict {
            files: vec!["README.md".to_string()]
        }
    );
    assert_eq!(snapshot(&work), before);
    assert_no_stash(&work);
}

#[test]
fn update_set_aside_untracked_name_clash_is_conflict_and_restores_exactly() {
    let (_tmp, work, other) = setup();
    fs::write(work.join("notes.txt"), "mine\n").unwrap();
    commit_file(&other, "notes.txt", "team\n", "team adds notes");
    git(&other, &["push"]);
    fetch(&work);
    let p = plan(&work).unwrap();
    assert_eq!(
        p,
        Plan::FastForward {
            downloads: 1,
            set_aside: true
        }
    );
    let before = snapshot(&work);

    let outcome = update(&work, &p).unwrap();

    assert_eq!(
        outcome,
        Outcome::Conflict {
            files: vec!["notes.txt".to_string()]
        }
    );
    assert_eq!(snapshot(&work), before);
    assert_no_stash(&work);
}

#[test]
fn update_combine_without_conflict_makes_merge_commit() {
    let (_tmp, work, other) = setup();
    commit_file(&work, "local.txt", "mine\n", "local change");
    push_new_commits(&other, 2);
    fetch(&work);
    let expected = plan(&work).unwrap();

    let outcome = update(&work, &expected).unwrap();

    assert!(
        matches!(outcome, Outcome::Updated { downloads: 2, .. }),
        "got {outcome:?}"
    );
    let parents = git(&work, &["rev-list", "--parents", "-n1", "HEAD"]);
    assert_eq!(
        parents.split_whitespace().count(),
        3,
        "HEAD is not a merge: {parents}"
    );
    assert_eq!(
        fs::read_to_string(work.join("local.txt")).unwrap(),
        "mine\n"
    );
    assert_no_stash(&work);
}

#[test]
fn update_combine_with_conflict_restores_exactly() {
    let (_tmp, work, other) = setup();
    commit_file(&work, "README.md", "mine\n", "my edit");
    commit_file(&other, "README.md", "theirs\n", "team edit");
    git(&other, &["push"]);
    fetch(&work);
    let expected = plan(&work).unwrap();
    let before = snapshot(&work);

    let outcome = update(&work, &expected).unwrap();

    assert_eq!(
        outcome,
        Outcome::Conflict {
            files: vec!["README.md".to_string()]
        }
    );
    assert_eq!(snapshot(&work), before);
    assert!(!merge_in_progress(&work), "MERGE_HEAD left behind");
    assert_no_stash(&work);
}

#[test]
fn update_plan_changed_does_nothing() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    fetch(&work);
    let old_plan = plan(&work).unwrap();
    push_new_commits(&other, 1);
    fetch(&work);
    let before = snapshot(&work);

    let outcome = update(&work, &old_plan).unwrap();

    assert_eq!(
        outcome,
        Outcome::PlanChanged(Plan::FastForward {
            downloads: 2,
            set_aside: false
        })
    );
    assert_eq!(snapshot(&work), before);
}

#[test]
fn update_on_detached_head_is_blocked() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    git(&work, &["checkout", "--detach"]);
    fetch(&work);
    let expected = plan(&work).unwrap();
    let before = snapshot(&work);

    let outcome = update(&work, &expected).unwrap();

    assert_eq!(outcome, Outcome::Blocked(Blocked::Detached));
    assert_eq!(snapshot(&work), before);
}

#[test]
fn update_with_up_to_date_plan_returns_up_to_date() {
    let (_tmp, work, _other) = setup();
    fetch(&work);
    let before = snapshot(&work);

    let outcome = update(&work, &Plan::UpToDate).unwrap();

    assert_eq!(outcome, Outcome::UpToDate);
    assert_eq!(snapshot(&work), before);
}

#[test]
fn update_keeps_other_local_branches_where_they_were() {
    let (_tmp, work, other) = setup();
    git(&work, &["branch", "keep"]);
    let keep_before = git(&work, &["rev-parse", "keep"]);
    push_new_commits(&other, 2);
    fetch(&work);
    let expected = plan(&work).unwrap();

    let outcome = update(&work, &expected).unwrap();

    assert!(
        matches!(outcome, Outcome::Updated { .. }),
        "got {outcome:?}"
    );
    assert_eq!(git(&work, &["rev-parse", "keep"]), keep_before);
    assert_eq!(
        git(&work, &["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
        "main"
    );
    assert_eq!(head(&work), origin_main(&work));
}

// ---- undo ----

#[test]
fn undo_after_fast_forward_restores_previous_head() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 2);
    fetch(&work);
    let old = head(&work);
    let expected = plan(&work).unwrap();
    let Outcome::Updated { undo: point, .. } = update(&work, &expected).unwrap() else {
        panic!("update did not succeed");
    };
    assert!(can_undo(&work, &point));

    let result = undo(&work, &point).unwrap();

    assert_eq!(result, UndoOutcome::Undone);
    assert_eq!(head(&work), old);
}

#[test]
fn undo_not_possible_after_user_makes_a_commit() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    fetch(&work);
    let expected = plan(&work).unwrap();
    let Outcome::Updated { undo: point, .. } = update(&work, &expected).unwrap() else {
        panic!("update did not succeed");
    };
    commit_file(&work, "mine.txt", "mine\n", "my own commit");
    let before = snapshot(&work);
    assert!(!can_undo(&work, &point));

    let result = undo(&work, &point).unwrap();

    assert_eq!(result, UndoOutcome::NotPossible);
    assert_eq!(snapshot(&work), before);
}

#[test]
fn undo_keeps_edit_to_file_the_update_did_not_touch() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    fetch(&work);
    let old = head(&work);
    let expected = plan(&work).unwrap();
    let Outcome::Updated { undo: point, .. } = update(&work, &expected).unwrap() else {
        panic!("update did not succeed");
    };
    fs::write(work.join("README.md"), "my edit after update\n").unwrap();

    let result = undo(&work, &point).unwrap();

    assert_eq!(result, UndoOutcome::Undone);
    assert_eq!(head(&work), old);
    assert_eq!(
        fs::read_to_string(work.join("README.md")).unwrap(),
        "my edit after update\n"
    );
}

#[test]
fn undo_not_possible_when_edit_touches_file_the_update_changed() {
    let (_tmp, work, other) = setup();
    commit_file(&other, "README.md", "team version\n", "team edit");
    git(&other, &["push"]);
    fetch(&work);
    let expected = plan(&work).unwrap();
    let Outcome::Updated { undo: point, .. } = update(&work, &expected).unwrap() else {
        panic!("update did not succeed");
    };
    fs::write(work.join("README.md"), "my edit after update\n").unwrap();
    let before = snapshot(&work);

    let result = undo(&work, &point).unwrap();

    assert_eq!(result, UndoOutcome::NotPossible);
    assert_eq!(snapshot(&work), before);
    assert_eq!(
        fs::read_to_string(work.join("README.md")).unwrap(),
        "my edit after update\n"
    );
}
