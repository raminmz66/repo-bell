# Check Progress Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While a check runs, a status strip under the window header and the tray's Check now item show which repo is being checked and how many of how many are done, and a check runs up to 4 repos at once.

**Architecture:** `check_repositories` (`watcher.rs`) runs watched repos through a small ordered worker pool (`run_in_order`, `std::thread::scope`, 4 threads) and reports `CheckProgress` steps through a callback. `check_once` stores each step in `EngineState.progress` and re-sends the whole state with the existing `publish`, which also changes only the tray's Check now item text. The window reads `state.progress`: `CheckStrip.svelte` sits between the header and the scrolling body, and a hidden status region in `App.svelte` says only start and end.

**Tech Stack:** Rust (Tauri 2, tokio `Mutex`, std threads), Svelte 5 (legacy `export let` / `$:` syntax, as the files use today), Vitest + Testing Library + `@tauri-apps/api/mocks`.

**Spec:** `docs/superpowers/specs/2026-11-17-check-progress-design.md` (read it in full; it has the copy, the strip look, the edge cases and the tests this plan refers to). Visual target: option C in `/home/ramin/projects/repo-bell/artifacts/check-progress.html`; tray: option B in `artifacts/check-progress-tray.html` (this worktree). Where they differ from the spec, the spec wins.

## Global Constraints

- Work only in `/home/ramin/projects/repo-bell/.claude/worktrees/check-progress`. Never touch the user's running RepoBell, its data in `~/.config/repobell` or `~/.local/state/repobell`, or `/var/www/html`.
- No new dependencies (no `rayon`). No new settings. No new timeouts (60 s fetch and 60 s server branch call stay).
- `const CHECK_WORKERS: usize = 4`, fixed.
- Keep the files' existing Svelte style (`export let`, `$:`, `onclick=`). Light theme only.
- Strip: Surface (#ffffff), 1px Hairline (#e3e3e0) bottom border, about 8px 16px 9px padding, about 40px high. Line 1 13px: "Checking" Soft Ink (#3d3d45), repo name Ink (#1b1b1f) 500, count Soft Ink with `font-variant-numeric: tabular-nums`. Name cut with an ellipsis, count never cut. Bar 6px below, 4px high, Chip Grey (#eeeeec) track, Muted Ink (#5b5b66) fill, 2px radius, width `done / total`, width eases 0.15s, no transition under `prefers-reduced-motion: reduce`. Never blue (One Blue Rule).
- Copy fixed by the spec: "Checking {name}", "{done} of {total}", tray "Checking {done} of {total}…" (one `…` character), "Check now", `aria-label="Check progress"`. Hidden status text "Checking your repos" / "Check finished" (wording may change only through `humanizer`).
- Every UI task: load the `impeccable` skill and read `/home/ramin/projects/repo-bell/.claude/skills/impeccable/reference/craft-floor.md` before the first UI edit. Follow DESIGN.md tokens. Where craft-floor disagrees with the spec, the spec wins.
- Every new user-facing string goes through the `humanizer` skill before commit.
- Builders write only the tests a task lists (ponytail mode). Test first.
- Rust: `export PATH="$HOME/.cargo/bin:$PATH"`, then from the worktree root `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` (as CI does).
- Frontend: `cd app && npm ci` once if `app/node_modules` is missing. Tests `cd app && npm test`, types `cd app && npm run check`.
- Commits: Conventional Commits, one short line, body ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. A step arrives while a news row is open and its Update preview shows: both stay open (rows and panels are keyed by path and the repos are re-sent unchanged). Test in Task 4.
2. A check that ends badly (worker panic, or results thrown away after a generation change) still clears `progress`, so the strip goes, the spinner stops and the tray reads "Check now" again. `check_once` has no test runtime; the builder of Task 2 clears it next to every `st.checking = false`, and the final hand check runs Settings save mid-check.
3. The tray menu is open while steps arrive: the label counts up and the menu neither closes nor flickers (only `set_text` / `set_enabled`, no `set_menu`). Hand check only.
4. A long repo name at 420px: it ends in an ellipsis and "6 of 15" stays whole. jsdom can't measure layout; hand check with a long-named repo.
5. An Update or Undo on a repo mid-check: that worker waits at the repo lock, the others go on, nothing hangs (a worker never holds a repo lock while it reports a step). Hand check.

---

### Task 1: Ordered worker pool and progress steps in `check_repositories` — complex

Label reason: threads, shared state and ordering logic.

**Files:**
- Modify: `app/src-tauri/src/watcher.rs` (new `CheckProgress`, `CHECK_WORKERS`, `run_in_order`; `check_repositories`; unit tests in `mod tests`)
- Test: `app/src-tauri/tests/watcher.rs` (pass a no-op callback everywhere; new tests)

**Interfaces:**
- Produces:
  - `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct CheckProgress { pub done: usize, pub total: usize, pub current: String }`
  - `pub const CHECK_WORKERS: usize = 4;`
  - `fn run_in_order<T: Sync, R: Send>(items: &[T], workers: usize, work: impl Fn(&T) -> R + Sync, on_step: impl Fn(usize, usize) + Sync) -> Vec<R>`: runs `work` on up to `workers` scoped threads; each thread takes the next unstarted index in order; results come back in item order. `on_step(done, first)` where `first` is the lowest index still in flight. One `std::sync::Mutex` holds the next index, the done count, the in-flight set and the last step sent; after every start and every finish, while holding it: if the in-flight set is not empty and `(done, first)` differs from the last step sent, call `on_step` and remember it. So the first start sends `(0, 0)`, each finish sends at most one step, and no step ever has `done == items.len()`. 0 items: no threads, no steps, empty result.
  - `pub fn check_repositories(settings: &Settings, seen: &Seen, undo: &BTreeMap<PathBuf, UndoPoint>, cleanup: &mut Cleanup, locks: &RepoLocks, on_progress: &(dyn Fn(CheckProgress) + Sync)) -> (Vec<RepoCardView>, bool)`
- Behaviour of `check_repositories`:
  - Discover as today. Watched = paths not in `settings.ignored`; only these go through `run_in_order(&watched, CHECK_WORKERS, …)`; `on_step(done, i)` becomes `on_progress(CheckProgress { done, total: watched.len(), current: repo_name(&watched[i]) })`.
  - Per watched repo (unchanged steps): `fetch_repo` without the lock; if it worked, `status` + `refresh_server_branches(&path, &main, &mut local, now_seconds(), false)` on a one-entry map `local` seeded from `cleanup.server.get(&path)` (the `refresh_old_branches` pattern in `actions.rs`); then `locks.get(&path).blocking_lock()` and `build_view` with a `Ctx` whose `cleanup` is a one-entry `Cleanup { server: local, undo: <this repo's undo entry> }`. Return `(view, local.remove(&path))`.
  - After the pool: insert every returned server entry into `cleanup.server` (entries for repos not checked stay). Assemble views in discovery order: ignored paths get `build_view(path, &ctx, None)` with the full `cleanup` (it returns the blank view at once, no Git). `is_offline` on the ordered list, as today.
  - Update the doc comment ("one after another" becomes "up to `CHECK_WORKERS` at a time").

- [ ] **Step 1: Write the failing tests**

  Unit tests in `watcher.rs` `mod tests` (sleeps are the work; keep margins wide):
  - `run_in_order_keeps_item_order`: items `0..6`, 4 workers, item 0 sleeps 80 ms, the rest 5 ms, `work` returns `i * 10` → result `[0, 10, 20, 30, 40, 50]`.
  - `run_in_order_runs_at_most_4_at_once`: 8 items, `CHECK_WORKERS`, each bumps an `AtomicUsize` in-flight counter, records the peak, sleeps 30 ms, drops the counter → peak `== 4`.
  - `run_in_order_steps_follow_the_oldest_in_flight`: 6 items, 4 workers, item 0 sleeps 150 ms, item 1 sleeps 300 ms, items 2 to 5 sleep 10 ms; collect steps in a `Mutex<Vec<(usize, usize)>>` → exactly `[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 1)]` (later repos finishing first don't move `first`; item 0 finishing moves it to 1; no `(6, _)`).
  - `run_in_order_with_no_items_sends_no_steps`: empty slice → empty result, no `on_step` call.

  Integration tests in `tests/watcher.rs` (import `CheckProgress`; every existing `check_repositories` call, including the `check` helper and the closure in the offline test, gains `&|_| {}`):
  - `progress_counts_watched_repos_only`: `repo_with_origin` for `a`, `b`, `c` (remote `/nonexistent/remote.git`), `b` in `settings.ignored`; collect steps in a `Mutex<Vec<CheckProgress>>` → views' names are `["a", "b", "c"]`; first step is `CheckProgress { done: 0, total: 2, current: "a".into() }`; every step has `total == 2` and `done < 2`; `done` never goes down.
  - `no_progress_without_watched_repos`: one repo, ignored → no steps (and an empty `folders` list → no steps).
  - `server_answers_of_parallel_repos_are_all_kept`: two `merged_feat()` setups (two temp dirs), one `Settings` with both watched folders, `git branch -D feat` on both remotes, one `check_repositories` with one `Cleanup` → `cleanup.server.len() == 2` and both views have `old_branches.len() == 1`.

- [ ] **Step 2: Run them to verify they fail**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p repobell`
Expected: does not compile (`run_in_order`, `CheckProgress` and the 6th argument don't exist).

- [ ] **Step 3: Implement** the interfaces above in `watcher.rs`.

- [ ] **Step 4: Run all checks**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: all pass, including `no_server_call_after_a_failed_fetch`, `server_answer_is_cached_for_an_hour`, `old_list_kept_when_server_fails_later` and the offline tests. `check_once` still compiles with a temporary `&|_| {}` (Task 2 replaces it).

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/watcher.rs app/src-tauri/tests/watcher.rs
git commit -m "feat(check): check up to 4 repos at once and report progress"
```

### Task 2: Progress in the app state and the tray item — complex

Label reason: shared state, a callback from worker threads, and tray branches.

**Files:**
- Modify: `app/src-tauri/src/watcher.rs` (`EngineState`, `AppStateView`, `to_view`, `check_once`, `apply_tray`, `publish`)
- Modify: `app/src-tauri/src/news.rs` (`check_now_label` and its test)

**Interfaces:**
- Consumes: `CheckProgress`, the `on_progress` argument of `check_repositories` (Task 1).
- Produces:
  - `AppStateView.progress: Option<CheckProgress>` (JSON `null` when no check runs), filled by `to_view` from `EngineState.progress`.
  - `EngineState` gains `pub progress: Option<CheckProgress>`, `pub check_item: Option<tauri::menu::MenuItem<tauri::Wry>>` and `pub check_label: Option<(String, bool)>` (the label and enabled flag last put on the item). All `None` in `new()`.
  - `news::check_now_label(progress: Option<&CheckProgress>) -> (String, bool)`: `None` → `("Check now", true)`; `Some(p)` → `(format!("Checking {} of {}…", p.done, p.total), false)`.
  - `apply_tray(app, tray, model, label: &(String, bool)) -> tauri::Result<MenuItem<tauri::Wry>>`: builds the Check now item with `MenuItem::with_id(app, "check_now", &label.0, label.1, None::<&str>)` in place of `.text("check_now", "Check now")` and returns it.
  - `publish`: `let label = news::check_now_label(st.progress.as_ref());` When the tray model changed, rebuild with `label` and store the item and label. Otherwise, when `st.check_label != Some(label)` and `st.check_item` is set, call `set_text` and `set_enabled` on it and store the label. `TrayModel` does not change. Then emit as today.
  - `check_once`: the blocking task calls `check_repositories(…, &on_progress)` where `on_progress` (cloned `AppHandle` and `SharedEngine`) does `let mut st = eng.state.blocking_lock(); st.progress = Some(p); publish(&app, &mut st);`. After the task, `st.progress = None` next to `st.checking = false`, before every `publish` on all three ways out (task failed, generation changed, results kept).

- [ ] **Step 1: Write the failing test** in `news.rs` `mod tests`:

```rust
#[test]
fn check_now_label_follows_progress() {
    assert_eq!(check_now_label(None), ("Check now".to_string(), true));
    let p = CheckProgress { done: 6, total: 15, current: "alpha".into() };
    assert_eq!(check_now_label(Some(&p)), ("Checking 6 of 15…".to_string(), false));
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p repobell --lib news::tests::check_now_label_follows_progress`
Expected: does not compile (`check_now_label` missing).

- [ ] **Step 3: Implement** the interfaces above. The rest is wiring with no Tauri test runtime in this crate; the final hand check covers it.

  `publish` now runs on a worker thread too. Tauri sends menu calls to the main thread, so it should just work. If the tray misbehaves in the running app, wrap only the tray part of a step in `app.run_on_main_thread`. If `MenuItem<Wry>` isn't `Send`, say so and stop (don't rebuild the menu per step instead).

- [ ] **Step 4: Run all checks**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: all pass, no warnings.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/watcher.rs app/src-tauri/src/news.rs
git commit -m "feat(check): send check progress to the window and tray"
```

### Task 3: Status strip in the window — complex

Label reason: touches 5 files.

Load `impeccable` and read `/home/ramin/projects/repo-bell/.claude/skills/impeccable/reference/craft-floor.md` before the first UI edit. Follow DESIGN.md tokens.

**Files:**
- Create: `app/src/lib/CheckStrip.svelte`
- Modify: `app/src/lib/types.ts`, `app/src/test-helpers.ts`, `app/src/App.svelte`
- Test: `app/src/__tests__/App.test.ts`

**Interfaces:**
- Consumes: `progress` in the state (Task 2).
- Produces:
  - `types.ts`: `export interface CheckProgress { done: number; total: number; current: string }` and `progress: CheckProgress | null` on `AppStateView`. `makeState` default `progress: null`.
  - `CheckStrip.svelte`, prop `export let progress: CheckProgress;`. Root `<div class="check-strip">` (not a live region). Line 1: `<span class="strip-what">Checking <span class="strip-name">{progress.current}</span></span>` and `<span class="strip-count">{progress.done} of {progress.total}</span>`. Bar: `<div class="strip-bar" role="progressbar" aria-label="Check progress" aria-valuemin="0" aria-valuemax={total} aria-valuenow={done} aria-valuetext="{done} of {total}">` holding `<div class="strip-fill" style:width="{(done / total) * 100}%">`. Look per Global Constraints.
  - `App.svelte`: in the main view only, right after `<Header … />` and before `<div class="main-body">`: `{#if state.progress}<CheckStrip progress={state.progress} />{/if}`. Not in Settings or the wizard (they are other branches).

- [ ] **Step 1: Write the failing tests** in a new `describe("check progress")` (`const progress = { done: 6, total: 15, current: "alpha" }`; `strip = () => document.querySelector(".check-strip")`):
  - `it.each` over 5 screens, each loaded through `get_app_state` with `progress` (this also covers a window opened mid-check): news view `newsState()`; compact All quiet (`makeState` with a side-branch repo `on_main: false, branch: "feature-x", main_behind: 2` plus a plain repo); big All quiet `makeState()`; offline `makeState({ offline: true })`; first run `makeState({ checking: true, last_checked: null })`. Each: `strip()` text contains "Checking alpha" and "6 of 15"; `getByRole("progressbar", { name: "Check progress" })` has `aria-valuemin "0"`, `aria-valuemax "15"`, `aria-valuenow "6"`, `aria-valuetext "6 of 15"`; `.strip-fill` has `style.width === "40%"`.
  - `shows the strip from a state-updated event and removes it when progress is null` (`shouldMockEvents: true`): start with `makeState()` → no strip; emit `makeState({ checking: true, progress })` → strip shows; emit `makeState({ progress: null })` → `strip()` is null.
  - `no strip in Settings`: state with `progress`, mock `detect_editors` → `["VS Code"]`, click Settings → `findByText("Folders and repos")`, then no progressbar.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/__tests__/App.test.ts`
Expected: the new tests fail.

- [ ] **Step 3: Implement** the interfaces above.

- [ ] **Step 4: Run all checks**

Run: `cd app && npm test && npm run check`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/CheckStrip.svelte app/src/lib/types.ts app/src/test-helpers.ts app/src/App.svelte app/src/__tests__/App.test.ts
git commit -m "feat(ui): show check progress in a strip under the header"
```

### Task 4: Headline text and the start and end announcement — complex

Label reason: adds state that tracks progress changes.

Load `impeccable` and read `/home/ramin/projects/repo-bell/.claude/skills/impeccable/reference/craft-floor.md` before the first UI edit.

**Files:**
- Modify: `app/src/App.svelte`
- Test: `app/src/__tests__/App.test.ts`

**Interfaces:**
- Consumes: `state.progress` (Task 3).
- Produces in App:
  - `subline` and `lastChecked` no longer use `state.checking`: `subline = checkedAgo ? \`Checked ${checkedAgo}\` : ""`, `lastChecked = checkedAgo ? \`Last checked ${checkedAgo}\` : ""`. Header spin, `isChecking`, the offline Try again "Checking…" and the first-run screen stay as they are.
  - `let checkStatus = ""; let wasChecking = false; function trackProgress(on: boolean)`: `on && !wasChecking` → `"Checking your repos"`; `!on && wasChecking` → `"Check finished"`; then `wasChecking = on`. Run as `$: trackProgress(!!state?.progress)`.
  - A visually hidden `<p class="check-status visually-hidden" role="status">{checkStatus}</p>` in the main view, right after the strip slot and outside it, always rendered (copy WelcomeView's `.visually-hidden` rule).

- [ ] **Step 1: Write the failing tests** (in `describe("check progress")`, `shouldMockEvents: true`; `status = () => document.querySelector(".check-status")`):
  - `subline keeps the last check time while checking`: the compact All quiet state from Task 3 with `checking: true, progress, last_checked: now - 120` → `.headline-row` text contains "Checked 2 min ago"; no element with text "Checking…".
  - `big All quiet keeps the last check time while checking`: `makeState({ checking: true, progress })` → `getByText(/Last checked 2 min ago\./)`; no "Checking…".
  - `announces only the start and the end`: start `makeState()` → `status()` exists with empty text; emit `{ checking: true, progress: { done: 0, total: 15, current: "alpha" } }` → text `"Checking your repos"`; emit `done: 1` → still `"Checking your repos"`; emit `{ checking: false, progress: null }` → `"Check finished"`. `status()` is never inside `.check-strip`.
  - `a step keeps an open row and its update preview` (Review Focus 1): one news repo with `unsaved: false` (opens by itself, ≤2 news), mock `preview_update` → the `fast_forward` plan used in this file; click "Update" → `findByRole("group", { name: "Update preview" })`; emit the same state with `checking: true, progress` → the row toggle is still `aria-expanded="true"` and the preview group is still there.
  - Keep `check_now that returns checking:true keeps the spinner until state-updated` unchanged.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/__tests__/App.test.ts`
Expected: the new tests fail (the subline says "Checking…", no status region).

- [ ] **Step 3: Implement** the interfaces above.

- [ ] **Step 4: Run all checks**

Run: `cd app && npm test && npm run check`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add app/src/App.svelte app/src/__tests__/App.test.ts
git commit -m "feat(ui): keep the last check time and announce only start and end"
```

### Task 5: Docs — simple

Run all new text through `humanizer`.

**Files:**
- Modify: `docs/decisions.md`, `DESIGN.md`, `ROADMAP.md`

- [ ] **Step 1: `docs/decisions.md`**: add the spec's "Check progress" section (from "Decision text for docs/decisions.md") as written, after "Main window layout" and before "## Design reviews". In "Checking for news", change "repos are checked one after another" to "up to 4 repos are checked at the same time".

- [ ] **Step 2: `DESIGN.md`**: add a "Status strip" entry under Layout (between the header band and the scrolling body: Surface, Hairline bottom border, 8px 16px 9px padding, 13px line with Soft Ink "Checking", Ink 500 name cut with an ellipsis, Soft Ink tabular count; 4px bar, Chip Grey track, Muted Ink fill, 2px radius, never blue). Add `check-strip` to the front matter `components`. In Motion, add the bar's 0.15s width ease, off under reduced motion.

- [ ] **Step 3: `ROADMAP.md`**: add under Fixes "Check progress and 4 repos at once" with Design, Plan and Build ticked and "Hand check in the running app, merge and reinstall" open. Set "Current step" to the build being done and waiting for the hand check and merge.

- [ ] **Step 4: Commit**

```bash
git add docs/decisions.md DESIGN.md ROADMAP.md
git commit -m "docs: record check progress and parallel checks"
```
