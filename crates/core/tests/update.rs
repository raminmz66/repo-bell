use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use repobell_core::update::{
    can_undo, plan, undo, update, update_main_ref, Blocked, MainRefOutcome, Outcome, Plan,
    UndoOutcome, UndoPoint,
};
use repobell_core::GitError;
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
    git(&work, &["branch", "--unset-upstream"]);
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
    let subject = git(&work, &["log", "-1", "--format=%s"]);
    assert_eq!(subject.trim(), "Merge remote-tracking branch 'origin/main'");
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

// ---- side branches ----

#[test]
fn plan_blocked_on_side_branch_and_update_changes_nothing() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    git(&work, &["switch", "-c", "feature"]);
    git(&work, &["push", "-u", "origin", "feature"]);
    fetch(&work);
    let before = snapshot(&work);

    assert_eq!(plan(&work).unwrap(), Plan::Blocked(Blocked::SideBranch));
    let outcome = update(&work, &Plan::Blocked(Blocked::SideBranch)).unwrap();

    assert_eq!(outcome, Outcome::Blocked(Blocked::SideBranch));
    assert_eq!(snapshot(&work), before);
}

#[test]
fn plan_blocked_when_main_tracks_something_else() {
    let (_tmp, work, _other) = setup();
    git(&work, &["push", "origin", "main:other"]);
    git(&work, &["branch", "--set-upstream-to=origin/other"]);

    assert_eq!(plan(&work).unwrap(), Plan::Blocked(Blocked::SideBranch));
}

fn on_side_branch_behind(work: &Path, other: &Path) {
    push_new_commits(other, 2);
    git(work, &["switch", "-c", "feature"]);
    commit_file(work, "mine.txt", "mine\n", "side work");
    fetch(work);
}

#[test]
fn update_main_ref_moves_main_without_touching_worktree() {
    let (_tmp, work, other) = setup();
    let old_main = head(&work);
    on_side_branch_behind(&work, &other);
    fs::write(work.join("README.md"), "unsaved edit\n").unwrap();
    fs::write(work.join("scratch.txt"), "x\n").unwrap();
    let before = snapshot(&work);

    let outcome = update_main_ref(&work).unwrap();

    assert_eq!(
        outcome,
        MainRefOutcome::Updated {
            from: old_main,
            to: origin_main(&work)
        }
    );
    assert_eq!(
        git(&work, &["rev-parse", "main"]).trim(),
        origin_main(&work)
    );
    assert_eq!(git(&work, &["branch", "--show-current"]).trim(), "feature");
    assert_eq!(snapshot(&work), before);
    assert_eq!(
        update_main_ref(&work).unwrap(),
        MainRefOutcome::AlreadyUpToDate
    );
}

#[test]
fn update_main_ref_refuses_when_main_has_local_commits() {
    let (_tmp, work, other) = setup();
    commit_file(&work, "local.txt", "l\n", "local main commit");
    let local = head(&work);
    git(&work, &["switch", "-c", "feature"]);
    push_new_commits(&other, 1);
    fetch(&work);

    assert_eq!(
        update_main_ref(&work).unwrap(),
        MainRefOutcome::MainHasLocalChanges
    );
    assert_eq!(git(&work, &["rev-parse", "main"]).trim(), local);
}

#[test]
fn update_main_ref_fails_when_main_is_checked_out() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    fetch(&work);
    let old = head(&work);

    let outcome = update_main_ref(&work).unwrap();

    assert!(matches!(outcome, MainRefOutcome::Failed(_)), "{outcome:?}");
    assert_eq!(head(&work), old);
}

// ---- ignored files ----

#[test]
fn update_refuses_to_overwrite_a_local_ignored_file() {
    let (_tmp, work, other) = setup();
    commit_file(&work, ".gitignore", ".env\n", "ignore env");
    git(&work, &["push"]);
    git(&other, &["pull"]);
    fs::write(other.join(".env"), "from team\n").unwrap();
    git(&other, &["add", "-f", ".env"]);
    git(&other, &["commit", "-m", "add env"]);
    git(&other, &["push"]);
    fs::write(work.join(".env"), "my secret\n").unwrap();
    fetch(&work);
    let expected = plan(&work).unwrap();
    let before = snapshot(&work);

    let outcome = update(&work, &expected).unwrap();

    assert_eq!(
        outcome,
        Outcome::Conflict {
            files: vec![".env".to_string()]
        }
    );
    assert_eq!(snapshot(&work), before);
}

#[test]
fn update_refuses_to_overwrite_an_ignored_file_that_arrives_by_rename() {
    let (_tmp, work, other) = setup();
    commit_file(&work, ".gitignore", ".env\n", "ignore env");
    fs::write(work.join("env.sample"), "SAMPLE=1\nA=2\nB=3\nC=4\n").unwrap();
    git(&work, &["add", "."]);
    git(&work, &["commit", "-m", "sample"]);
    git(&work, &["push"]);
    git(&other, &["pull"]);
    git(&other, &["mv", "env.sample", ".env"]);
    git(&other, &["add", "-f", ".env"]);
    git(&other, &["commit", "-m", "rename sample to env"]);
    git(&other, &["push"]);
    fs::write(work.join(".env"), "my secret\n").unwrap();
    fetch(&work);
    let expected = plan(&work).unwrap();
    let before = snapshot(&work);

    let outcome = update(&work, &expected).unwrap();

    assert_eq!(
        outcome,
        Outcome::Conflict {
            files: vec![".env".to_string()]
        }
    );
    assert_eq!(snapshot(&work), before);
}

// ---- stash pop keeps the index ----

#[test]
fn update_set_aside_keeps_staged_and_unstaged_apart() {
    let (_tmp, work, other) = setup();
    commit_file(&work, "a.txt", "a\n", "a");
    commit_file(&work, "b.txt", "b\n", "b");
    git(&work, &["push"]);
    git(&other, &["pull"]);
    push_new_commits(&other, 1);
    fs::write(work.join("a.txt"), "staged edit\n").unwrap();
    git(&work, &["add", "a.txt"]);
    fs::write(work.join("b.txt"), "unstaged edit\n").unwrap();
    fetch(&work);
    let expected = plan(&work).unwrap();

    let outcome = update(&work, &expected).unwrap();

    assert!(matches!(outcome, Outcome::Updated { .. }), "{outcome:?}");
    assert_eq!(git(&work, &["diff", "--cached", "--name-only"]), "a.txt\n");
    assert_eq!(git(&work, &["diff", "--name-only"]), "b.txt\n");
}

// ---- rollback safety ----

/// A post-merge hook that creates `.git/index.lock` and, if `release_after` is set, removes it later.
fn lock_after_merge(work: &Path, release_after: Option<&str>) {
    use std::os::unix::fs::PermissionsExt;
    let release = release_after
        .map(|s| format!("(sleep {s}; rm -f \"$L\") >/dev/null 2>&1 &"))
        .unwrap_or_default();
    let hook = work.join(".git/hooks/post-merge");
    fs::write(
        &hook,
        format!(
            "#!/bin/sh\nL=\"$(git rev-parse --git-dir)/index.lock\"\ntouch \"$L\"\n{release}\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn update_retries_when_index_lock_is_briefly_held() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    fs::write(work.join("README.md"), "my unsaved edit\n").unwrap();
    fetch(&work);
    let expected = plan(&work).unwrap();
    lock_after_merge(&work, Some("0.6"));

    let outcome = update(&work, &expected).unwrap();

    assert!(matches!(outcome, Outcome::Updated { .. }), "{outcome:?}");
    assert_eq!(
        fs::read_to_string(work.join("README.md")).unwrap(),
        "my unsaved edit\n"
    );
    assert_no_stash(&work);
}

#[test]
fn update_reports_edits_in_stash_when_rollback_cannot_run() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    fs::write(work.join("README.md"), "my unsaved edit\n").unwrap();
    fetch(&work);
    let expected = plan(&work).unwrap();
    lock_after_merge(&work, None);

    let result = update(&work, &expected);

    let Err(GitError::EditsInStash { stash, moved, .. }) = result else {
        panic!("expected EditsInStash, got {result:?}");
    };
    assert!(
        moved,
        "the merge went through, so HEAD is already on the team's version"
    );
    fs::remove_file(work.join(".git/index.lock")).unwrap();
    assert_eq!(git(&work, &["rev-parse", "stash@{0}"]).trim(), stash);
    assert_eq!(
        git(&work, &["show", &format!("{stash}:README.md")]),
        "my unsaved edit\n"
    );
}

#[test]
fn undo_rejects_a_before_that_is_not_a_full_sha() {
    let (_tmp, work, other) = setup();
    push_new_commits(&other, 1);
    fetch(&work);
    let old = head(&work);
    let expected = plan(&work).unwrap();
    let Outcome::Updated { undo: point, .. } = update(&work, &expected).unwrap() else {
        panic!("update did not succeed");
    };
    for bad in [&old[..7], "HEAD~1", "--hard"] {
        let bad = UndoPoint {
            before: bad.to_string(),
            after: point.after.clone(),
        };
        assert_eq!(undo(&work, &bad).unwrap(), UndoOutcome::NotPossible);
    }
    assert_eq!(head(&work), point.after);
}

#[test]
fn stash_ref_finds_the_entry_by_sha() {
    let (_t, work, _other) = setup();
    fs::write(work.join("a.txt"), "one").unwrap();
    git(&work, &["add", "a.txt"]);
    git(&work, &["stash", "push", "-m", "first"]);
    let first = git(&work, &["rev-parse", "stash@{0}"]).trim().to_string();
    fs::write(work.join("b.txt"), "two").unwrap();
    git(&work, &["add", "b.txt"]);
    git(&work, &["stash", "push", "-m", "second"]);
    assert_eq!(
        repobell_core::stash_ref(&work, &first).as_deref(),
        Some("stash@{1}")
    );
    assert_eq!(repobell_core::stash_ref(&work, "deadbeef"), None);
}
