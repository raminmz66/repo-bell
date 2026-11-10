# Branch Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each repo card offers to delete local branches the team already deleted on the server, with Undo.

**Architecture:** A new core module `crates/core/src/branches.rs` finds old branches from local data plus one cached `git ls-remote --heads origin` answer, and deletes and restores them. The Tauri watcher keeps the server answer cache and the cleanup Undo data in `EngineState`, adds the list to `RepoCardView`, and exposes three commands. `RepoCard.svelte` shows the quiet line, the list and the Undo banner.

**Tech Stack:** Rust (repobell_core, Tauri 2), Svelte 5 + Vitest + Testing Library, Git CLI (Ubuntu 22.04 ships Git 2.34.1).

**Spec:** `docs/superpowers/specs/2026-11-10-branch-cleanup-design.md`

## Global Constraints

- Local only: never delete or push anything on the server.
- A branch is listed only when all hold: it has an upstream on remote `origin`; that upstream is not `refs/heads/<main>`; the upstream branch is absent from the server's branch list; it is not the local main branch; it is not checked out in the main checkout or any linked worktree.
- Merged = branch tip is an ancestor of `origin/<main>`. Merged rows start ticked; "may not be merged" rows start unticked.
- Cleanup never counts as news: no change to `new_count`, the tray dot, tray menu, notifications, `error`/`error_kind`.
- `git ls-remote --heads origin` runs at most once per 3600 seconds per repo during checks, plus every time the user opens the list, and only for repos with at least one candidate branch. It uses the same SSH env, no-hooks rule and 60 s timeout as `fetch_main`. On failure keep the last good answer; with none, no cleanup line.
- Delete only when the branch still points at the sha the user saw. Delete and restore are user actions (`git::run`); listing is background (`run_background` / `run_with_timeout`).
- Branch names reach Git only as separate arguments, as full refs (`refs/heads/<name>`), never through a shell.
- Undo recreates each branch at its sha with its `branch.<name>.remote` and `branch.<name>.merge`; a name that exists again is skipped and reported. Undo data lives in memory, like Update's Undo, and stays while at least one deleted name is still free.
- No "are you sure?" prompt before deleting.
- All user-facing copy goes through the `humanizer` skill before commit.
- Commit messages: Conventional Commits, short, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. Branch names with slashes or non-ASCII (`feature/login`, `fix/ünïcode`): listed, deleted and restored correctly. Test in Task 1 and Task 2.
2. Repo with no `origin` remote, or a remote that is not called `origin`: no list, no error on the card. Test in Task 1.
3. Detached HEAD: listing still works, nothing crashes. Test in Task 1.
4. Server unreachable on the very first check: card has no cleanup line and no error; after a good answer, a later failure keeps the old list. Test in Task 3.
5. Branch deleted by hand in a terminal between listing and Delete: reported as skipped, the rest still deleted. Test in Task 2.

---

### Task 1: Find old branches (core)

**Files:**
- Create: `crates/core/src/branches.rs`
- Modify: `crates/core/src/lib.rs` (add `pub mod branches;` and re-exports)
- Modify: `crates/core/src/fetch.rs` (make `ssh_env` `pub(crate)`)
- Test: `crates/core/tests/branches.rs`

**Interfaces:**
- Produces:
  - `pub struct OldBranch { pub name: String, pub sha: String, pub merged: bool, pub last_commit_time: i64 }` (derive `Debug, Clone, PartialEq, Eq`)
  - `pub fn has_candidates(repo: &Path, main_branch: &str) -> Result<bool, GitError>`: true when at least one local branch passes every listing rule except the server one. Local only.
  - `pub fn server_branches(repo: &Path) -> Result<BTreeSet<String>, GitError>`: full refs such as `"refs/heads/feat"` from `git ls-remote --heads origin`.
  - `pub fn old_branches(repo: &Path, main_branch: &str, server: &BTreeSet<String>) -> Result<Vec<OldBranch>, GitError>`: listed branches, merged first, then by name.

- [ ] **Step 1: Write the failing tests** in `crates/core/tests/branches.rs`, using `common::setup_remote_and_clone` (`work` is the user, `other` the team) and a local helper `old(work) -> Vec<(String, bool)>` that calls `server_branches` then `old_branches(work, "main", &server)` and maps to `(name, merged)`:
  - `merged_branch_deleted_on_server_is_listed_as_merged`: in `work`, push `feat` with a commit, fast-forward main to it and push, delete `feat` on the server (`git push origin --delete feat`), `git fetch`. Assert `old(work) == [("feat", true)]`.
  - `squash_merged_branch_is_listed_as_not_merged`: push `sq`, squash-merge into main from `other` and push, delete `sq` on server, fetch in `work`. Assert `[("sq", false)]`.
  - `branch_still_on_server_is_not_listed`: push `develop`, merge it into main and push, keep it on the server. Assert empty.
  - `never_pushed_branch_is_not_listed`: `git branch local-only` with a commit, no upstream. Assert empty, and `has_candidates == false`.
  - `branch_tracking_main_is_not_listed`: `git checkout -b fresh origin/main`, then `git checkout main`. Assert empty.
  - `checked_out_branch_is_not_listed`: gone-on-server branch that is checked out in `work`. Assert empty.
  - `worktree_branch_is_not_listed`: gone-on-server branch checked out with `git worktree add ../wt <name>`. Assert empty.
  - `slash_and_unicode_names_are_listed`: `feature/login` and `fix/ünï` both gone on server. Assert both listed.
  - `no_origin_is_empty_not_error`: repo from `common::init_repo` with a branch. `has_candidates` returns `Ok(false)`; `server_branches` returns `Err` (callers treat it as "no answer").
  - `detached_head_works`: gone branch, then `git checkout --detach`. Assert it is listed.
  - `has_candidates_true_before_server_check`: pushed `feat` still on server: `has_candidates == true`, `old_branches` empty.
  - `last_commit_time_is_the_tip_commit_time`: assert it equals `git log -1 --format=%ct feat`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p repobell-core --test branches`
Expected: compile error, `branches` module not found.

- [ ] **Step 3: Implement `branches.rs`**

Read local branches with one call:
`git for-each-ref --format=%(refname:strip=2)%00%(objectname)%00%(upstream:remotename)%00%(upstream:remoteref)%00%(committerdate:unix) refs/heads` (verified on Git 2.34.1: the fields come back as `name|sha|origin|refs/heads/name|time`, empty upstream fields for unpushed branches). Read checked-out branches from `git worktree list --porcelain` (`branch refs/heads/<name>` lines; this includes the main checkout). Both through `run_background`. Merged check: `git merge-base --is-ancestor <sha> refs/remotes/origin/<main>` (exit 1 = not merged, other failures = error). `server_branches` uses `git::run_with_timeout(repo, &["ls-remote", "--heads", "origin"], &crate::fetch::ssh_env(repo), Duration::from_secs(60))` and takes the second tab field of each line.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p repobell-core --test branches`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "feat(core): find branches the team deleted on the server"
```

### Task 2: Delete and restore branches (core)

**Files:**
- Modify: `crates/core/src/branches.rs`, `crates/core/src/lib.rs`
- Test: `crates/core/tests/branches.rs`

**Interfaces:**
- Consumes: `old_branches`, `OldBranch` from Task 1.
- Produces:
  - `pub struct DeletedBranch { pub name: String, pub sha: String, pub remote: String, pub merge: String }` (derive `Debug, Clone, PartialEq, Eq`; `remote`/`merge` are the old `branch.<name>.remote` / `.merge` values)
  - `pub struct DeleteOutcome { pub deleted: Vec<DeletedBranch>, pub skipped: Vec<String> }`
  - `pub struct RestoreOutcome { pub restored: Vec<String>, pub skipped: Vec<String> }`
  - `pub fn delete_branches(repo: &Path, main_branch: &str, picks: &[(String, String)], server: &BTreeSet<String>) -> Result<DeleteOutcome, GitError>`: `picks` are `(name, sha)` the user saw.
  - `pub fn restore_branches(repo: &Path, deleted: &[DeletedBranch]) -> Result<RestoreOutcome, GitError>`
  - `pub fn can_restore(repo: &Path, deleted: &[DeletedBranch]) -> bool`: true when at least one name is free.

- [ ] **Step 1: Write the failing tests**
  - `delete_removes_branch_and_its_config`: gone `feat`; delete `[("feat", sha)]`. Assert `deleted.len() == 1`, `git rev-parse --verify -q refs/heads/feat` fails, `git config --get branch.feat.merge` fails.
  - `delete_skips_branch_that_moved`: list, then commit on `feat`, delete with the old sha. Assert `skipped == ["feat"]`, branch still there.
  - `delete_skips_names_not_in_the_list`: picks for `main`, a checked-out branch, a worktree branch, a branch still on the server and a made-up name. Assert all in `skipped`, all real branches still exist.
  - `delete_skips_branch_removed_by_hand`: list two gone branches, `git branch -D` one, delete both. Assert one deleted, one skipped.
  - `restore_brings_back_sha_and_upstream`: delete then restore. Assert `restored == ["feat"]`, `rev-parse feat == sha`, `branch.feat.remote == "origin"`, `branch.feat.merge == "refs/heads/feat"`.
  - `restore_skips_taken_name`: delete, create a new `feat`, restore. Assert `skipped == ["feat"]` and the new branch is untouched. `can_restore` is false.
  - `restore_slash_unicode_names`: delete and restore `feature/login` and `fix/ünï`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p repobell-core --test branches`
Expected: compile errors for the new functions.

- [ ] **Step 3: Implement**

`delete_branches` lists `old_branches(repo, main_branch, server)` again and acts only on picks whose `(name, sha)` match a listed row; the rest go to `skipped`. Before deleting, read `branch.<name>.remote` and `branch.<name>.merge` with `git config --get`. Delete with `git::run(repo, &["update-ref", "-d", &format!("refs/heads/{name}"), sha])`; a `Failed` error means skipped. Then `git config --remove-section branch.<name>` (ignore failure). Restore: `git update-ref refs/heads/<name> <sha> ""` (empty old value = create only; verified it refuses an existing ref on 2.34.1), then set both config values. A failed create = skipped.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p repobell-core`
Expected: all pass, including the existing suites.

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "feat(core): delete and restore old branches"
```

### Task 3: Old branches on the card (watcher)

**Files:**
- Modify: `app/src-tauri/src/watcher.rs`
- Test: `app/src-tauri/tests/watcher.rs`

**Interfaces:**
- Consumes: `has_candidates`, `server_branches`, `old_branches`, `can_restore`, `DeletedBranch` from Tasks 1–2.
- Produces:
  - `pub struct OldBranchView { pub name: String, pub sha: String, pub merged: bool, pub last_commit_time: i64 }` (Serialize/Deserialize)
  - `RepoCardView` gains `pub old_branches: Vec<OldBranchView>` and `pub can_undo_cleanup: bool` (both `Default`).
  - `pub struct ServerBranches { pub checked_at: u64, pub heads: BTreeSet<String> }`
  - `#[derive(Default, Clone)] pub struct Cleanup { pub server: BTreeMap<PathBuf, ServerBranches>, pub undo: BTreeMap<PathBuf, Vec<DeletedBranch>> }`; `EngineState` gains `pub cleanup: Cleanup`.
  - `pub const SERVER_BRANCHES_MAX_AGE: u64 = 3600;`
  - `pub fn refresh_server_branches(path: &Path, main_branch: &str, cache: &mut BTreeMap<PathBuf, ServerBranches>, now: u64, force: bool)`: runs `server_branches` when `has_candidates` and (`force` or the entry is missing or older than `SERVER_BRANCHES_MAX_AGE`); on `Ok` stores it, on `Err` keeps the old entry. Network, so no repo lock.
  - `Ctx` gains `pub cleanup: &'a Cleanup`. `build_view` fills `old_branches` from `ctx.cleanup.server` (empty when no entry, or when the card has `error`) and `can_undo_cleanup` from `can_restore`.
  - `check_repositories(settings, seen, undo, cleanup: &mut Cleanup, locks)`: calls `refresh_server_branches(.., now_seconds(), false)` next to `fetch_repo`, before taking the lock. `check_once` passes a clone of `st.cleanup` and, when the generation still matches, writes back `server` and keeps `undo` entries only for repos whose card has `can_undo_cleanup` or an error. `refresh_one` passes `&st.cleanup` into `Ctx`.

- [ ] **Step 1: Write the failing tests** in `app/src-tauri/tests/watcher.rs` with the file's existing helpers:
  - `card_lists_branch_deleted_on_server`: push and merge `feat`, delete it on the server. After `check_repositories`, the card has one `old_branches` row, `merged == true`, and `new_count`, `error` are unchanged from a run without the branch.
  - `server_answer_is_cached_for_an_hour`: run a check, then delete a second branch on the server, run again with the same `Cleanup`: still one row. Set the entry's `checked_at` back by 3601 s, run again: two rows.
  - `no_line_when_server_never_answered`: remote URL set to a missing path before the first check: `old_branches` empty, card error is the fetch error only (no extra error from cleanup).
  - `old_list_kept_when_server_fails_later`: good check first, then break the remote URL, check again: the row is still there.
  - `no_server_call_without_candidates`: repo with only main and a never-pushed branch: after a check, `cleanup.server` has no entry for it.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p repobell --test watcher`
Expected: compile errors for the new fields and the new `check_repositories` argument.

- [ ] **Step 3: Implement the interfaces above in `watcher.rs`** and update every existing `check_repositories` / `Ctx` call site (watcher.rs, actions.rs, tests) to pass the new argument.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri
git commit -m "feat(app): show old branches on the repo card"
```

### Task 4: Delete, Undo and refresh commands

**Files:**
- Modify: `app/src-tauri/src/actions.rs`, `app/src-tauri/src/lib.rs` (register commands)
- Test: `app/src-tauri/src/actions.rs` `#[cfg(test)]` for the message builder

**Interfaces:**
- Consumes: Task 2 core functions, Task 3 `Cleanup`, `refresh_server_branches`, `refresh_one`.
- Produces (all `#[tauri::command]`, each starts with `ensure_watched` and takes the repo lock like `undo_repo` does, except for the network call):
  - `refresh_old_branches(app, path: PathBuf, engine) -> Result<(), String>`: `refresh_server_branches(.., force = true)` without the lock, writes the entry into `st.cleanup.server`, then takes the lock and calls `refresh_one`.
  - `#[derive(Deserialize)] pub struct BranchPick { pub name: String, pub sha: String }`
  - `delete_old_branches(app, path: PathBuf, branches: Vec<BranchPick>, engine) -> Result<SimpleResult, String>`: uses the cached server set for that repo (none cached = `SimpleResult::new("not_possible", ..)`), calls `delete_branches`, stores `deleted` in `st.cleanup.undo[path]` (replacing older data), bumps `generation`, calls `refresh_one`. Status `"deleted"` when anything was deleted, else `"not_possible"`.
  - `undo_cleanup(app, path: PathBuf, engine) -> Result<SimpleResult, String>`: `restore_branches` with the stored list, removes it, `refresh_one`. Status `"undone"` or `"not_possible"`.
  - `fn cleanup_message(done: usize, skipped: &[String], verb: CleanupVerb) -> String` builds the copy below.

Copy (run through `humanizer` before commit; keep the meaning):
- Deleted: "Deleted 3 branches. You can undo it." / "Deleted 1 branch. You can undo it."
- Skipped on delete, appended: "Skipped feat-x because it changed since you looked." (several: "Skipped 2 branches because they changed since you looked: a, b.")
- Nothing deleted: "Nothing was deleted. These branches changed since you looked."
- Undone: "Brought back 3 branches." Skipped on undo, appended: "Skipped feat-x because a branch with that name exists again."
- Nothing to undo: "There's nothing to undo."

- [ ] **Step 1: Write the failing unit tests** for `cleanup_message`: one deleted, several deleted, deleted plus one skipped, nothing deleted, undone with a skip. Assert the exact strings above.

- [ ] **Step 2: Run them to verify they fail**

Run: `cargo test -p repobell cleanup_message`
Expected: compile error.

- [ ] **Step 3: Implement the commands and register them in `lib.rs`'s `invoke_handler`.**

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri
git commit -m "feat(app): delete old branches with undo"
```

### Task 5: Card UI and docs

**Files:**
- Modify: `app/src/lib/types.ts`, `app/src/test-helpers.ts` (`old_branches: []`, `can_undo_cleanup: false` defaults), `app/src/lib/RepoCard.svelte`
- Test: `app/src/lib/__tests__/RepoCard.test.ts`
- Modify: `docs/decisions.md`, `ROADMAP.md`, `README.md`

**Interfaces:**
- Consumes: `RepoCardView.old_branches`, `can_undo_cleanup`; commands `refresh_old_branches({path})`, `delete_old_branches({path, branches: [{name, sha}]})`, `undo_cleanup({path})` returning `SimpleResult`.
- Produces: `export interface OldBranch { name: string; sha: string; merged: boolean; last_commit_time: number }` in `types.ts`.

UI rules:
- A quiet text button "4 old branches" / "1 old branch" with the other quiet status lines, shown only when `old_branches.length > 0` and there is no `error`. Title: "Your team deleted these branches on the server. You can delete your copies." It never renders the news badge.
- Clicking it toggles an inline list on the card and calls `refresh_old_branches`. Group "Merged into main" (checkboxes ticked) and group "May not be merged" (unticked, with the hint "The server no longer has these, but Git can't find them in main. Tick only the ones you're done with."). Each row: checkbox labelled with the branch name, plus `last change {timeAgo(last_commit_time)}` using the existing `timeAgo` in `util.ts` (it takes Unix seconds and returns "3 d ago"). Show a group only when it has rows.
- Button "Delete 3 branches" / "Delete 1 branch"; disabled and labelled "Delete branches" when nothing is ticked. While running: "Deleting…".
- When `repo.can_undo_cleanup`, show a banner in the existing `.undo-banner` style with the last result text (or "RepoBell deleted branches here. You can undo it.") and an "Undo" button calling `undo_cleanup`. Use the existing `notice` pattern for results.
- A compact up-to-date card with no old branches stays compact with no buttons (the existing test must still pass).

- [ ] **Step 1: Write the failing tests** in `RepoCard.test.ts` with `mockIPC`:
  - `shows "2 old branches" line and no news badge`: repo with two rows, `new_count: 0`. Assert the text, and `queryByText(/\bnew\b/)` is null.
  - `hides the line when the card has an error`.
  - `opening the list asks for a fresh server answer`: click the line, assert `refresh_old_branches` was invoked with `{ path: PATH }`.
  - `merged rows start ticked and may-not-be-merged rows start unticked`.
  - `delete sends only the ticked branches with their sha`: tick one unmerged row too, click "Delete 2 branches", assert the payload `branches` equals both `{name, sha}`.
  - `delete button is disabled when nothing is ticked`.
  - `undo banner shows when can_undo_cleanup and calls undo_cleanup`.
  - `slash names render and are sent unchanged` (`feature/login`).

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/lib/__tests__/RepoCard.test.ts`
Expected: the new tests fail.

- [ ] **Step 3: Implement the types, helpers and card changes.**

- [ ] **Step 4: Run all checks**

Run: `cd app && npm test && npm run check`
Expected: all pass, no svelte-check errors.

- [ ] **Step 5: Docs**

Add a "Branch cleanup" section to `docs/decisions.md` with the spec's rules (listing rules, merged vs may not be merged, server check and cache, local only, Undo, no confirm prompt), and date it 2026-11-10. In `ROADMAP.md`, tick the Phase 5 items and set "Current step". Add one line to `README.md` under "What It Does": old branch cleanup, and that GitHub's "Automatically delete head branches" setting makes it work best. Run all new text through `humanizer`.

- [ ] **Step 6: Run the app and check by hand**

Run: `cd app && npm run tauri dev`. In a test repo with one merged and one squash-merged branch deleted on the server, confirm: the line shows, the groups and ticks are right, Delete works, Undo brings both back, the tray dot does not change. Take a screenshot on display `:1`.

- [ ] **Step 7: Commit**

```bash
git add app/src docs ROADMAP.md README.md
git commit -m "feat(ui): branch cleanup on the repo card"
```
