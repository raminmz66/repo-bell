# Check progress and parallel checks design

Date: 2026-11-17. Status: draft for Ramin's review. Ramin decided the strip, the 4-at-once rule and the tray text on 2026-11-17. Everything marked "(Claude's call)" is open to change.

Visual targets: option C "Status strip under the header" in `artifacts/check-progress.html` (main checkout) and option B "Menu shows progress" in `artifacts/check-progress-tray.html` (this worktree). Where this spec and a mockup differ, the spec wins.

## Why

After "Check now" (the refresh icon top left, or "Check now" in the tray) the user waits blind. The only signs are the small spinning header icon and "Checking…" under the headline. There is no count, no repo name and no sense of how long is left.

The wait is long. Measured on Ramin's machine on 2026-11-17 with 15 watched repos under `/var/www/html`: one `git ls-remote --heads origin` takes about 4 s per repo. 15 in a row took 58 s, 4 at once 15 s, 8 at once 7 s. A real check runs `fetch_main` per repo plus, for repos with old-branch candidates, a second server call at most once an hour, so a check today takes 1 to 2 minutes.

## Goals

- While a check runs, every main-window screen shows which repo it is on, how many are done out of how many, and a bar that fills per finished repo.
- The tray menu shows the same count, so a tray click visibly did something.
- A check of 15 repos takes about a quarter of today's time.
- A slow repo is easy to spot: its name stays in the strip.

## Non-goals

- A setting for how many repos run at once.
- A cancel button, time left, or per-row "checking" states.
- A "check done" notification.
- New timeouts. The existing ones stay: 60 s for a fetch and 60 s for the server branch call (`crates/core`).

## Anti-goals

The change fails, even if it works, if any of these happen:

- The bar moves without a repo finishing (fake progress).
- The screen reader reads out every step.
- An open row, update preview or alert closes or resets during a check.
- Results land in a different order than today, or a repo's old-branch list is lost.
- The tray menu closes or flickers on every step.

## Approaches

Ramin picked the look in the mockups on 2026-11-17:

- Window: A (count in the headline plus a thin bar), B (each row shows its own state) or C (status strip under the header). C picked: same place on every screen, names the repo. A has no headline line on the All quiet and Can't check screens and doesn't name the repo. B hides most rows in the closed Up to date list, and faded rows look broken.
- Tray: A (as today), B (Check now becomes "Checking 6 of 15…", greyed out) or C (B plus a "done" notification). B picked. C adds noise, because RepoBell already notifies when there is real news.
- Speed: 4 repos at once, fixed. About 4 times faster, and well under SSH's default `MaxStartups` (10).

How the progress reaches the window (Claude's call):

1. A `progress` field in the state the window already gets (picked). Each step re-sends the state with the existing `publish`. One path: a window opened mid-check gets the strip from `get_app_state` with no extra code. The cost is about 16 state messages per check for 15 repos, which is small.
2. A separate small `check-progress` event. Lighter messages, but the window then needs a second listener, and still needs the field for a window opened mid-check.

How the tray item changes (Claude's call):

1. Change only the Check now item's text and enabled flag with `MenuItem::set_text` and `set_enabled` (picked). The 2026-11-06 tray test showed text updates live, even with the menu open.
2. Put the label in `TrayModel`, so the whole menu is rebuilt per finished repo. Not tested with the menu open, and may close or flicker it.

How the 4 run at once (Claude's call): `std::thread::scope` with 4 worker threads inside the existing blocking task, each taking the next repo in order. No new dependency. `rayon` would add one for the same thing, and one tokio task per repo needs a semaphore plus a blocking task each.

## What the user sees

### Status strip (window)

- It sits directly under the header and above the scrolling body, so it never scrolls away. It is white (Surface) with a 1px Hairline bottom border, about 8px 16px 9px padding and about 40px high. It pushes the content down while it shows (accepted).
- Line 1, 13px: "Checking repo-name" on the left, "6 of 15" on the right. "Checking" is Soft Ink, the repo name is Ink at weight 500, the count is Soft Ink with tabular figures so it doesn't jiggle. A long repo name is cut with "..."; the count is never cut.
- Under it, 6px down, a 4px bar: Chip Grey track, Muted Ink fill, 2px radius. Never blue (One Blue Rule: progress is plain status). Its width is `done / total`.
- The repo name is the one that has been running longest among the 4 in flight. A repo that hangs stays named until it finishes or times out, while the count keeps going.
- It shows on every main-window screen while a check runs: the news view, the compact and the big All quiet, the Can't check (offline) screen during Try again, and the first-run "Checking your repos…" screen. It does not show in Settings or the welcome wizard. (Claude's call: those screens aren't about the list.)
- It shows from the first step ("0 of 15", bar empty) and goes away when the results land. It never shows "15 of 15": the last finish is not sent on its own, and the next state already has no progress. (Claude's call: that full bar would last a few milliseconds.)

### Headline and buttons

- The subline under the headline no longer says "Checking…". During a check it keeps "Checked 4 min ago" from the last finished check, as in mockup C, because the strip now says a check is running. After the check it reads "Checked just now", as today. The big All quiet text keeps "Last checked 4 min ago." the same way. (Claude's call.)
- The header icon keeps spinning and stays disabled while a check runs, as today. (Claude's call: it explains why the button can't be pressed, it covers the moment between the click and the first step, and it needs no change.)
- The offline screen's Try again button keeps "Checking…" while disabled, as today.

### Tray menu

- While a check runs, "Check now" reads "Checking 6 of 15…" and is greyed out. The number moves with each finished repo, even with the menu open.
- When the check ends it reads "Check now" again and works.
- The same happens for timer checks, not only for a click.
- No notification at the end.

## How it works

### Progress data

- New `progress` field in `AppStateView` (and `types.ts`): `{ done, total, current }` or `null`.
  - `total`: the repos this check fetches, so watched (not ignored) repos found by discovery. Ignored repos still get their blank row as today but are not counted. Discovery is quick and local and runs before the count exists.
  - `done`: repos fully finished (fetch, server branch call when due, and reading the result).
  - `current`: the name (the row name, `RepoCardView.name`) of the in-flight repo that started first. Repos start in discovery order, so it is the lowest-position repo still running.
- `progress` is `null` when no check runs, and also when a check finds 0 watched repos. It is set only while at least one repo is in flight, so `current` is always set when `progress` is not `null`.
- `EngineState` gets `progress: Option<…>`. `check_once` clears it on every way out: results kept, results thrown away after a generation change, and a failed blocking task (a worker panic). Then it publishes as today.
- Each step publishes the full state with the existing `publish`. A step is sent only when `done` or `current` changes. In practice that is the first start and then one per finished repo.

### Parallel check

`check_repositories` keeps its job and its `&mut Cleanup` argument, and gains a progress callback (for example `on_progress: &(dyn Fn(Progress) + Sync)`). Existing tests pass a no-op.

- Discover the repos as today. Ignored repos skip the network as today.
- Watched repos run on up to 4 worker threads (`const CHECK_WORKERS: usize = 4`), each taking the next repo in discovery order.
- Per repo, unchanged steps: `fetch_repo` without the repo's lock. If the fetch worked, `refresh_server_branches` (still at most once an hour per repo); if it failed, no server branch call. Then take the repo's lock with `blocking_lock` and call `build_view`.
- The shared server branch map: each worker starts from a one-entry map holding its repo's cached answer, the same pattern `refresh_old_branches` in `actions.rs` already uses. Its `build_view` sees that repo's fresh answer. After all workers finish, the answers are inserted into `cleanup.server`, so the map ends the same as today's sequential run. Entries for repos not checked this time stay, as today.
- Results go back in discovery order (store by position, not by finish time). Offline detection (`is_offline`) runs on the ordered list, unchanged.
- Progress order: the done count and in-flight set live behind one mutex inside the check, and the callback is called while holding it, so steps never arrive out of order. The callback in `check_once` takes the state lock with `blocking_lock`, sets `progress`, and publishes. Lock order stays safe: a worker never holds a repo lock while it reports a step, and nothing else takes the progress mutex.
- `publish` from a worker thread emits the event and touches the tray from a non-async thread. The builder confirms this in the running app (Tauri runs menu calls on the main thread) If the tray misbehaves, wrap only the tray part of the step in `app.run_on_main_thread`. (From the builder check.)

### Tray item

- `apply_tray` builds the Check now item with the current label and enabled flag, and keeps a handle to it (in `EngineState`, or looked up by id; builder's call).
- `publish` works out the label from `progress`: `None` gives "Check now" and enabled; `Some` gives "Checking {done} of {total}…" and disabled. It calls `set_text` and `set_enabled` only when the label changed. `TrayModel` does not change, so the menu is not rebuilt per step.
- A small pure function returns the label and enabled flag, so it can be unit tested.

### Re-runs

- `run_check`'s "again" loop and a thrown-away check (generation change) start a new pass. The strip restarts at "0 of N" with a fresh discovery, so `total` may change. The strip may vanish for a moment between passes. Accepted. (Claude's call.)

## Edge cases

| Case | What happens |
|---|---|
| 0 watched repos (no folders, or all ignored) | No progress. No strip; tray keeps "Check now". |
| 1 repo | "Checking app · 0 of 1", empty bar, then gone. |
| Check already running when the user clicks | The header button, Try again and the tray item are disabled while it runs, so nothing to click. A click in the short gap before the first step sets "again" as today, and one more pass runs after. |
| Window opened mid-check | `get_app_state` returns `progress`, so the strip shows at once. While hidden, the window keeps getting `state-updated` as today. |
| Every repo fails fast (offline) | Failed fetches skip the server call, so the count runs up quickly, then the strip goes and the Can't check screen shows as today. |
| A slow or hanging repo | Its name stays in the strip while the others finish and the count goes on. It ends at the existing 60 s fetch timeout (plus up to 60 s for the server branch call). No new timeout. |
| Update, Undo or delete on a repo mid-check | That repo's worker waits at the repo lock as today; the other workers go on. |
| Settings saved or Mark all as seen mid-check | The generation changes, results are thrown away as today, and a new pass restarts the strip. |
| A step arrives while a row or panel is open | Each step re-sends the old repos unchanged. Rows, open feeds, previews and alerts must not reset (tested). |

## Keyboard and screen reader

- The strip itself is not a live region, so steps are not read out.
- The bar is `role="progressbar"` with `aria-valuemin="0"`, `aria-valuemax={total}`, `aria-valuenow={done}`, `aria-valuetext="6 of 15"` and `aria-label="Check progress"`, so a screen reader user who moves to it hears the count on demand.
- A separate visually hidden `role="status"` element in the main view is always present. It says "Checking your repos" when a check starts and "Check finished" when it ends, and nothing in between. It must be in the page before the text changes, so it can't live inside the strip. (Claude's call.) It follows `progress` exactly, so timer checks while the window is open, and the gap between two passes, also announce start and end. Accepted: the window is usually open for a short time. (Claude's call, from the builder check.)
- The bar width eases over 0.15s. Under `prefers-reduced-motion: reduce` there is no transition.
- The strip has no buttons, so tab order doesn't change.

## Tests

Rust (`app/src-tauri`):

- Parallel order: with more repos than workers and the first repo slowest, results come back in discovery order. Use a small pure helper for the worker pool if that makes the test possible without Git (for example one that runs a closure per item with a sleep). Builder's call on the shape.
- Never more than 4 at once, and 4 at once when there are 8 items (count the peak in flight).
- Progress steps: `done` only goes up, `total` is the watched count, ignored repos aren't counted, `current` is the lowest-position repo in flight (a later repo finishing first doesn't change it; the first one finishing moves it to the next), and no step has `done == total`.
- 0 watched repos: no steps.
- Server map merge: two repos with old-branch candidates both have entries in `cleanup.server` after one check. Existing tests keep passing, including `no_server_call_after_a_failed_fetch`, `server_answer_is_cached_for_an_hour` and the offline tests in `tests/watcher.rs`.
- Tray label: `None` gives "Check now" and enabled; `done 6, total 15` gives "Checking 6 of 15…" and disabled.

Svelte (Vitest and Testing Library), `App.test.ts`:

- The strip shows "Checking alpha" and "6 of 15" with the progressbar values, on each screen: news view, compact All quiet, big All quiet, offline, and the first-run "Checking your repos…" screen.
- A `state-updated` with `progress: null` removes the strip.
- `get_app_state` returning `progress` (window opened mid-check) shows the strip with no event.
- No strip in Settings while `progress` is set.
- The subline keeps "Checked 2 min ago" while `checking` is true; the big All quiet keeps "Last checked 2 min ago." Update the tests that expect "Checking…" there.
- The status region reads "Checking your repos" after the first step, keeps the same text after the next step, and reads "Check finished" after `progress` goes back to `null`.
- An open news row and an open update preview are still open after a progress step.
- The existing "check_now that returns checking:true keeps the spinner until state-updated" test stays.

Before finishing, the builder runs the real app against the 15 repos: times a check, takes a screenshot of the strip at 420×640, and watches the tray item change with the menu open.

## Out of scope

- A worker count setting, a cancel button, time left, per-row progress, a "done" notification.
- New or changed timeouts.
- The header icon's spin has no reduced-motion rule today. Separate fix.
- A timer check that fires during a running check still adds one more pass, as today.
- The "Later" offline banner in `ROADMAP.md`.
- Settings and the welcome wizard.

## Left to the builder

- The exact shape of the progress callback and the worker pool helper, as long as the tests pass.
- Where the tray item handle lives.
- Exact spacing within DESIGN.md tokens.
- Final wording of the hidden status text. Text the user reads goes through the `humanizer` skill.

## Decision text for docs/decisions.md

Add when built:

> ## Check progress
>
> Decided 2026-11-17. Full design: `docs/superpowers/specs/2026-11-17-check-progress-design.md`.
>
> While a check runs, a thin strip under the window header shows "Checking repo-name" and "6 of 15", with a grey bar that fills one step per finished repo. It shows on every main-window screen and goes away when the check ends. The name is the repo that has been running longest, so a slow one stands out. The bar is grey because progress is not news. Screen readers hear only the start and the end.
>
> In the tray, "Check now" becomes "Checking 6 of 15…" and is greyed out until the check ends, for timer checks too. Only that item's text changes, so the menu isn't rebuilt per repo. There is no "done" notification, because RepoBell already notifies when there is real news.
>
> RepoBell checks up to 4 repos at the same time, in discovery order, with results kept in that order. Measured on 2026-11-17 with 15 repos: 58 s one after another, 15 s with 4 at once. 4 stays well under SSH's default limit of 10 new connections. There is no setting for it.

Also change "repos are checked one after another" in "Checking for news" to "up to 4 repos are checked at the same time".

## Docs to update when built

- `docs/decisions.md`: the section above and the "Checking for news" sentence.
- `DESIGN.md`: the status strip and its bar (Chip Grey track, Muted Ink fill).
- `ROADMAP.md`: the new item and the current step.
