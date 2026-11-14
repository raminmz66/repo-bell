# Grouped Rows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The main window shows repos as one- or two-line rows in groups (Can't check, News, On main, On a branch, Up to date) instead of cards, so a glance at 420×640 shows everything that needs the user.

**Architecture:** `App.svelte` sorts repos into groups, renders each with one snippet, and owns which rows are open (the Q3 rule, the new `window-shown` event and the tray focus jump). `RepoCard.svelte` keeps all its script logic but renders as an `<li>` row with an `expanded` prop; its panel state lives in a `Map` that App owns, so a row that moves to another group keeps its panels. `FeedList.svelte` becomes a flat list of 3 titles. The backend only gains one `window-shown` event in `lib.rs`.

**Tech Stack:** Svelte 5 (legacy `export let` / `$:` syntax, as the files use today), Vitest + Testing Library + `@tauri-apps/api/mocks`, Rust (Tauri 2).

**Spec:** `docs/superpowers/specs/2026-11-14-grouped-rows-design.md` (read it in full; it has the copy, the row table and every rule this plan refers to).

## Global Constraints

- Work only in `/home/ramin/projects/repo-bell/.claude/worktrees/improve-ux`. Never touch the user's running RepoBell, its data in `~/.config/repobell` or `~/.local/state/repobell`, or `/var/www/html`.
- No new dependencies. No new backend data. The only backend change is the `window-shown` event (Task 1).
- Keep the files' existing Svelte style (`export let`, `$:`, `onclick=`, snippets). Light theme only.
- Styling uses DESIGN.md tokens only: label 12px 700 uppercase Soft Ink (#3d3d45) with the count in Muted Ink (#5b5b66); row name 14px 700; line 2 13px Muted Ink; flags 12px chips with 6px radius; row padding about 9px 12px; group list white, 10px radius, 1px Hairline (#e3e3e0) border and dividers. Row buttons are 32px tall (decided, below DESIGN.md's 36px).
- Row states: openable row hover #fbfbf9 and pointer cursor; open row keeps #fbfbf9; flash = inset 2px Signal Blue (#1f5fd1) ring with #f4f8fe fill; conflict or stash alert = inset 1.5px #c4372b ring. Chevron and flash transitions off under `prefers-reduced-motion`.
- Every UI task: load the `impeccable` skill and read `/home/ramin/projects/repo-bell/.claude/skills/impeccable/reference/craft-floor.md` before the first UI edit. Where craft-floor disagrees with the spec, the spec wins (the uppercase group label is the section's `<h2>`, not a kicker).
- Every new user-facing string goes through the `humanizer` skill before commit. Copy fixed by the spec stays as written.
- No `alert()`, `confirm()` or `prompt()` (`no-alert.test.ts` scans for it). No `role="button"` anywhere.
- Builders write only the tests a task lists (ponytail mode). Test first where the task says so.
- Frontend: `cd app && npm ci` once if `app/node_modules` is missing. Tests `cd app && npm test`, types `cd app && npm run check`.
- Rust: `export PATH="$HOME/.cargo/bin:$PATH"`, then from the worktree root `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`.
- Commits: Conventional Commits, one short line, body ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. A row that leaves News (after Update or a background check) keeps its open state in its new group. Test in Task 7.
2. With 2 or fewer news repos, a row the user closed stays closed when a `state-updated` event arrives; only rows newly entering News open. Test in Task 7.
3. A tray focus jump to a row that can't open (a Can't check repo) still scrolls and flashes it, with no toggle and no error. Test in Task 7.
4. "Mark all as seen" on the News label that fails shows the window banner, as the footer button did. Test in Task 6.
5. A long repo name, a long branch name and a long error reason at 420px: the name ends in an ellipsis with a hover, the branch and the error wrap and are never cut. jsdom can't measure layout, so this is checked in the Task 8 hand check with fixtures that have long names.

---

### Task 1: `window-shown` event (backend) — complex

Label reason: adds a branch.

**Files:**
- Modify: `app/src-tauri/src/lib.rs` (`show_and_position_window`)

**Interfaces:**
- Produces: the window emits event `"window-shown"` (payload `()`) only when `show_and_position_window` found the window hidden. It is emitted at the end of the function, so on a tray repo line it fires before the `focus-repo` emit that follows the call.

- [ ] **Step 1: Implement.** At the top: `let was_hidden = !window.is_visible().unwrap_or(true);` (an error counts as visible, so no reset). At the end, after `set_focus`: `if was_hidden { let _ = window.emit("window-shown", ()); }`. Update the doc comment in one line. `Emitter` is already imported.

  No automated test: it needs a real window (no Tauri test runtime in this crate). Task 8 checks it in a running dev instance.

- [ ] **Step 2: Check**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --workspace && cargo clippy --workspace -- -D warnings`
Expected: all pass, no warnings.

- [ ] **Step 3: Commit**

```bash
git add app/src-tauri/src/lib.rs
git commit -m "feat(app): tell the window when it is shown again"
```

### Task 2: Row open state and panel memory move out of RepoCard — complex

Label reason: state logic, reactive save and restore.

Load `impeccable` and read craft-floor.md before editing the `.svelte` file.

**Files:**
- Modify: `app/src/lib/RepoCard.svelte` (script and the disclosure button only; the layout rewrite is Task 4)
- Test: `app/src/lib/__tests__/RepoCard.test.ts`

**Interfaces:**
- Produces (new RepoCard props and one exported type):
  - `export let expanded: boolean = false;` the feed shows when `expandable && expanded`.
  - `export let onToggle: () => void = () => {};` the disclosure button calls it and no longer flips local state.
  - `export let panels: Map<string, PanelState> = new Map();` the row restores its panel state from `panels.get(repo.path)` at init and writes it back with a `$:` statement whenever any of it changes (not in `onDestroy`: the row in the new group can mount before the old one is destroyed).
  - `export let onAlertChange: (path: string, on: boolean) => void = () => {};` called from `$: onAlertChange(repo.path, hasAlert)`.
  - `export type PanelState` in a `<script module lang="ts">` block: `{ plan, notice, noticeFrom, conflictFiles, conflictMessage, stashAlert, stashRef, stashCommand, mainDetail, showDetail, fileError, actionError, showBranches, choice }` with today's types (move the `Notice` type into the module block too). Not `busy`.
  - `hasAlert = plan !== null || conflictFiles.length > 0 || stashAlert !== null || notice?.kind === "warn" || notice?.kind === "error" || actionError !== null`. Success and info notices don't count.
- Removes: `isExpanded`, `prevNew`, `trackNews`, and the expand line in `reveal()` (it keeps the scroll).

- [ ] **Step 1: Change the tests first**
  - Replace "starts a news card expanded and a quiet one with own changes collapsed" with `follows the expanded prop`: `newsRepo()` with no `expanded` → toggle `aria-expanded="false"` and no "Fix login timeout"; `expanded: true` → `"true"` and the title shows.
  - "uses a real disclosure button inside the heading, with no div role=button": pass `onToggle = vi.fn()`, `expanded: true`; click the toggle → `onToggle` called once; `rerender({ expanded: false })` → title gone. Keep the `role="button"` and heading checks (still `h2` here).
  - "expands and scrolls into view when highlighted" becomes `scrolls into view and flashes when highlighted`: scroll called, root has class `highlight`, toggle stays `aria-expanded="false"`.
  - Add `expanded: true` to every feed test that reads titles or files: "shows change titles but never commit hashes", "shows at most 3 titles…", "shows no 'and N more'…", "groups by person…", "lists files and opens one…", "shows a failed editor open inline".
  - New `keeps panel state across a remount`: `const panels = new Map()`; render `newsRepo({ unsaved: true })` with `panels`; mock `preview_update` → `setAside`, `update_repo` → `result({ status: "edits_in_stash", message: msg, stash: "stash@{0}", stash_command: "git stash pop" })`; run Update → "Set aside and update"; wait for `msg`; `unmount()`; render again with the same `panels` → `msg` is in the document.
  - New `reports when an alert panel shows`: `onAlertChange = vi.fn()`; same stash flow → last call `(PATH, true)`; click "Close" → last call `(PATH, false)`. Then a second render with `update_repo` → `result({})` (success) → last call `(PATH, false)`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/lib/__tests__/RepoCard.test.ts`
Expected: the changed and new tests fail.

- [ ] **Step 3: Implement the interfaces above.** Init each panel variable from `saved?.x ?? <today's default>`.

- [ ] **Step 4: Run all checks**

Run: `cd app && npm test && npm run check`
Expected: all pass (App tests don't read the feed, so they still pass with rows closed).

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/RepoCard.svelte app/src/lib/__tests__/RepoCard.test.ts
git commit -m "refactor(ui): let the window own row open state and panel memory"
```

### Task 3: Flat feed list — complex

Label reason: touches 6 files and adds the author-prefix branch.

Load `impeccable` and read craft-floor.md before editing.

**Files:**
- Modify: `app/src/lib/FeedList.svelte`, `app/src/lib/RepoCard.svelte` (stop passing `now` to FeedList), `app/src/lib/types.ts` (remove `DayGroup`, `AuthorDayGroup`)
- Delete: `app/src/lib/feedGrouping.ts`, `app/src/lib/feedGrouping.test.ts`
- Test: `app/src/lib/__tests__/RepoCard.test.ts`

**Interfaces:**
- Produces: `FeedList` props are `feed: FeedSummary`, `repoPath: string` (the `now` prop is removed). It renders:
  - A `<ul>` of the newest 3 titles (today's sort and `MAX_TITLES`), wrapped, never cut. When the shown titles have more than one author (`mine` counts as the author "You"), each `<li>` text is `"<author>: <title>"` in one text node; with one author, just the title.
  - Then one line: `<span>{more} more</span>` (left out when `more` is 0), `" · "` only when both parts show, then today's "N files changed" toggle button with its file list and error text unchanged (left out with no files). No line when both are absent.
  - No summary line, no time, no day labels, no author headers, no left border.

- [ ] **Step 1: Change the tests first** (all with `expanded: true`)
  - "shows at most 3 titles and then 'and N more'" becomes `shows at most 3 titles and then "N more · N files changed"`: same 5 changes by Sara, `total_changes: 8` → "One", "Three" show, "Four" doesn't, `screen.getByText("5 more")`, and its parent element's `textContent` with whitespace collapsed is `"5 more · 2 files changed"`.
  - "shows no 'and N more' when everything fits": `queryByText(/\d+ more/)` is null; the "2 files changed" button still shows.
  - "groups by person when several people changed things" becomes `prefixes the author only when titles have more than one author`: Sara "A", Ali "B", mine "C" → `getByText("Sara: A")`, `getByText("Ali: B")`, `getByText("You: C")`, and `queryByText("Sara")` is null (no author header). Single author (`makeFeed()`) → `getByText("Fix login timeout")` and no `getByText(/^Alex:/)`.
  - New `has no summary line or day labels`: `queryByText("Alex added 2 changes to the main version")`, `queryByText("Today")` and `queryByText("Yesterday")` are null.
  - "uses a real disclosure button…": check the title "Fix login timeout" instead of the summary line.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/lib/__tests__/RepoCard.test.ts`
Expected: the changed tests fail.

- [ ] **Step 3: Implement** the FeedList output above, remove the `now` pass in RepoCard, delete `feedGrouping.ts` and its test, remove the two types.

- [ ] **Step 4: Run all checks**

Run: `cd app && npm test && npm run check`
Expected: all pass; `grep -rn "feedGrouping\|DayGroup" app/src` prints nothing.

- [ ] **Step 5: Commit**

```bash
git add -A app/src/lib
git commit -m "feat(ui): show the feed as a flat list of the latest titles"
```

### Task 4: RepoCard as a two-line row — complex

Label reason: touches 4 files and adds per-row branching for line 2, flags and actions.

Load `impeccable` and read craft-floor.md before editing. Run new copy (hover texts) through `humanizer`.

**Files:**
- Modify: `app/src/lib/RepoCard.svelte` (markup and styles), `app/src/lib/util.ts`
- Test: `app/src/lib/__tests__/RepoCard.test.ts`, `app/src/__tests__/App.test.ts` (only the assertions this markup change breaks)

**Interfaces:**
- Consumes: Task 2 props, Task 3 FeedList.
- Produces:
  - `export function shortTimeAgo(timestamp: number | null, now?: number): string` in `util.ts`: `timeAgo` without `" ago"`, and `"just now"` → `"now"`. Built on `timeAgo`, not a copy of it.
  - RepoCard root is `<li class="row" data-repo-path={repo.path}>`, keeping `class:highlight`. Repo name is in an `<h3>`. Classes `compact`, `badge-err`, `.hint` per card, and the "Up to date" / "Main is up to date" / "You're on …" texts are gone.

Layout (see the spec's "Row" section and table for every case):
- `.row-head` (position: relative) holds line 1 and line 2. Line 1: when `expandable`, one `<button class="toggle">` inside the `<h3>` holding the chevron, the name and (news rows) the "N new" badge with today's title, `aria-expanded={expanded}`, `aria-controls={bodyId}`, `onclick={onToggle}`. Its `::after` (`position: absolute; inset: 0`) covers both lines. When not expandable, the name is plain text in the `<h3>`. The name has `title={repo.name}` and an ellipsis. Right side: news rows show `shortTimeAgo(feed.newest_time, now)` with `title="Newest change {timeAgo(...)}"`, then the action. Action buttons, flags and the old-branches flag get `position: relative; z-index: 1` so they sit above the hit area.
- Actions (32px): Update (outline) when `canUpdate` and no plan or conflict; Update main as a text button (Signal Blue text, no border, Blue Wash hover) when `canUpdateMain`, with `title="Refreshes main in the background. Your files stay as they are."`; error rows: Try again (outline) or Remove (secondary), today's logic.
- Line 2 (13px muted; text first, then flags; flags wrap to a new line, the text never gets an ellipsis). Leave line 2 out when it has no text and no flags.
  - Error: `repo.error`.
  - News: authors from `feed.changes` (distinct, newest first, `mine` → "You"; up to 2 names joined by ", ", then ` and N other(s)`), then ` · N file(s)` when `files_changed` is not empty. One `<span>` for the whole text.
  - On main, behind, no news: today's "Behind by N" with its title.
  - On a branch, no news: `"{branch ?? 'Not on a branch'} · main N behind"` (from `main_behind`) or `"… · main up to date"`.
  - Otherwise no text.
- Flags in this order: branch flag (news rows not on main only: the branch name, or "Not on a branch"; grey chip), "Unsaved" (amber, `title="You edited files that aren't saved as a change yet"`), "N old branches" (the existing toggle button with its title, `aria-expanded`, `aria-controls`, styled as a grey chip; no error rows), "Ahead by N" (grey, on main and `ahead > 0`, today's title).
- Under `.row-head`, outside the hit area, in today's order: the open feed (`expandable && expanded`), the old-branch list (moved out of the flag), the update preview, stash alert, conflict list, both Undo banners, notices, `actionError`. The panels themselves are unchanged.
- Red ring class when `conflictFiles.length > 0 || stashAlert !== null`.

- [ ] **Step 1: Write and change the tests first** in `RepoCard.test.ts`:
  - Rewrite "shows the news count as the badge and Behind as a quiet status next to Update" as `news row: badge with hover, no Behind`: `newsRepo({ behind: 4, new_count: 3 })` → "3 new" has title "Your team made 3 changes since you last looked", `queryByText(/Behind by/)` null, Update button present.
  - "has no news badge when only behind": also `getByText("Behind by 2")` has title "The team made 2 changes you don't have yet".
  - New `news line 2 names authors and files`: changes by Sara (newest) and Amine, 2 files → `getByText("Sara, Amine · 2 files")`; a mine change → text starts with "You"; one file → `"… · 1 file"`; `files_changed: []` → `getByText("Sara, Amine")`; authors Sara, Amine, Bo, Cy → `"Sara, Amine and 2 others · 2 files"`; Sara, Amine, Bo → `"Sara, Amine and 1 other · 2 files"`.
  - "shows Ahead with hover text and Has unsaved changes": `getByText("Unsaved")` with its title; Ahead unchanged.
  - New `old branches flag opens the list without toggling the row`: `onToggle = vi.fn()`, repo with `feed: makeFeed()` and two old branches, `mockIPC(() => {})` → click "2 old branches" → it has `aria-expanded="true"`, "Merged into main" shows, `onToggle` not called.
  - New `only the toggle toggles the row`: `onToggle = vi.fn()`, `newsRepo()`, `expanded: true`, `mockIPC` with `preview_update` → `fastForward` → click Update, the title "Fix login timeout", and the preview's "Cancel" → `onToggle` not called; click the toggle → called once.
  - "renders an up-to-date repo as a compact row…" becomes `up-to-date row has no button and no status text`: `makeRepo()` → name shows, no button, `queryByText("Up to date")` null, no `.compact`.
  - New `a row that can't open has no toggle`: error repo and `makeRepo()` → no `aria-expanded` element except old-branches flags; `container.querySelector('[role="button"]')` null; the name is inside an `h3`.
  - "uses a real disclosure button inside the heading…": `closest("h3")`.
  - New `shows the newest change time in short form`: `newsRepo()` (makeFeed's newest is 300 s ago) → `getByText("5 min")` with title "Newest change 5 min ago". In `util` terms: `shortTimeAgo(now/1000 - 10)` is "now".
  - Side branch: "names the branch and offers Update main" → `getByText("feature-x · main 3 behind")`, Update main has the hint as `title`, the hint text is not in the document as text, no Behind or Ahead text. "shows a quiet note…" → `getByText("feature-x · main up to date")`, no Update main. "says so when HEAD isn't on a branch" → `makeRepo({ on_main: false, branch: null, main_behind: 3 })` → `getByText("Not on a branch · main 3 behind")`.
  - New `news row on a side branch`: `newsRepo({ on_main: false, branch: "feature-x", main_behind: 2 })` → Update main button, no Update button, `getByText("feature-x")` (the flag), authors text present.
  - Error card: "shows the reason, a Can't check badge and Try again that checks" → `queryByText("Can't check")` null; rest unchanged.

  In `App.test.ts`, only what this markup breaks: "uses the backend headline…" expects `queryByText("Behind by 4")` null; "keeps errors in their own section…" uses heading level 3 for "docs" and `/^web-app/`, `closest("li")`, and drops the `getByText("Can't check")` line (Task 6 adds the label check); "tucks plain up-to-date repos…" finds the row with `document.querySelector('[data-repo-path="/home/user/projects/quiet-one"]')`, expects no `compact` class, no "Up to date" text inside the row and no button in it; "keeps repos with something to act on visible…" expects `getByText("feature-x · main 2 behind")`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/lib/__tests__/RepoCard.test.ts src/__tests__/App.test.ts`
Expected: the new and changed tests fail.

- [ ] **Step 3: Implement** `shortTimeAgo` and the row layout above. Keep every script function as it is.

- [ ] **Step 4: Run all checks**

Run: `cd app && npm test && npm run check`
Expected: all pass. (Rows are `<li>` inside App's old `<div>` lists until Task 6; jsdom doesn't mind.)

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/RepoCard.svelte app/src/lib/util.ts app/src/lib/__tests__/RepoCard.test.ts app/src/__tests__/App.test.ts
git commit -m "feat(ui): show each repo as a two-line row"
```

### Task 5: Missing folder as a row — simple

Load `impeccable` and read craft-floor.md before editing.

**Files:**
- Modify: `app/src/lib/FolderErrorCard.svelte` (markup and styles only)

**Interfaces:**
- Produces: root `<li class="row" data-folder-path={folder.path}>`, same props and script. Line 1: the folder name in an `<h3>` (plain text, ellipsis, `title` with the name) and Remove (secondary, 32px) on the right. Line 2: the message, wrapped. Line 3: the full path (mono 12px, muted, ellipsis, `title` = path). The failed-remove text under it. The "Can't check" badge is gone. Same padding, name and line-2 styles as a RepoCard row.

- [ ] **Step 1: Implement.** No new tests; the existing App tests "shows a Folder not found card…" and "shows a failed Remove on the card…" cover its behavior.

- [ ] **Step 2: Run all checks**

Run: `cd app && npm test && npm run check`
Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add app/src/lib/FolderErrorCard.svelte
git commit -m "feat(ui): show a missing folder as a row"
```

### Task 6: Groups, labels and Mark all as seen on the News label — complex

Label reason: touches 4 files and adds grouping logic and alert routing.

Load `impeccable` and read craft-floor.md before editing. Run new copy through `humanizer`.

**Files:**
- Modify: `app/src/App.svelte`, `app/src/lib/Footer.svelte`
- Test: `app/src/__tests__/App.test.ts`, `app/src/lib/__tests__/Footer.test.ts`

**Interfaces:**
- Consumes: RepoCard props `panels`, `onAlertChange` (Task 2); rows as `<li>` (Tasks 4, 5).
- Produces in App:
  - `const panels = new Map<string, PanelState>()` (import the type from `./lib/RepoCard.svelte`), passed to every RepoCard.
  - `let alertPaths = new Set<string>()` and `function setAlert(path: string, on: boolean)` that reassigns the set only when membership changes (otherwise the reactive call loops).
  - Groups, each repo in exactly one, checked in this order: `errorRepos` (`r.error`); `newsRepos` (`r.new_count > 0`, any branch); `onBranch` (`!r.on_main`); `upToDate` (`isPlain(r) && !alertPaths.has(r.path)`, `isPlain` unchanged); `onMain` (the rest).
  - `{#snippet rowList(rows: RepoCardView[], folders: FolderError[] = [])}`: a `<ul class="group-list">` of RepoCards, then FolderErrorCards. Every RepoCard gets `repo`, `now`, `checking={isChecking}`, `highlight`, `panels`, `onAlertChange={setAlert}`, `onTryAgain={handleCheckNow}`, `onStopWatching={stopWatching}`, `onStateRefresh={loadState}`.
  - `{#snippet group(label: string, rows: RepoCardView[], extra: { hint?: string; folders?: FolderError[]; markSeen?: boolean } = {})}`: renders nothing when `rows.length + (extra.folders?.length ?? 0)` is 0; else `<section>` with a label row (`<h2>{label} <span class="count">{count}</span></h2>`, plus a "Mark all as seen" text button on the right when `markSeen`, calling `handleMarkAllSeen`), the hint line when given, then `rowList`.
  - The accordion keeps today's button, label and count, and opens to `rowList(upToDate)`.
  - Headline branch order: headline row, `group("Can't check", errorRepos, { folders: folderErrors })`, `group("News", newsRepos, { markSeen: true })`, then the shared quiet part: `group("On main", onMain)`, `group("On a branch", onBranch, { hint: "Update main refreshes main in the background. Your files stay as they are." })`, accordion. All quiet branch: the unchanged message, then the shared quiet part. `allPlain` = no On main, no On a branch and no errors.
  - Headline subline: `"Checking…"` while checking, else `"Checked {timeAgo}"`, else empty. "Since you last looked" is gone. The All quiet text keeps "Last checked …".
  - Remove `section-title`, `errorList`, `quietList`, `cards-list` and the Footer's `hasNews` / `onMarkAllSeen` props.
- Footer: only "Add folder"; props `onAddFolder` only.

- [ ] **Step 1: Write and change the tests first**
  - App, new `renders groups in order with counts`: state with an error repo, one folder error, one news repo, one on-main repo with `behind: 2`, one side-branch repo and one plain repo → `getAllByRole("heading", { level: 2 })` text, whitespace collapsed, is `["Can't check 2", "News 1", "On main 1", "On a branch 1"]`; accordion button `/Up to date 1 repo/`. A state with only one news repo has no `heading` named `/^On main/` or `/^On a branch/`.
  - App, "Mark all as seen calls mark_all_seen…" moves to `describe("news and errors")`: find the button with `within(screen.getByRole("heading", { level: 2, name: /^News/ }).closest("section")!)`; after the click, "All quiet". Also `within(document.querySelector("footer")!).queryByRole("button", { name: "Mark all as seen" })` is null.
  - App, new `shows a failed Mark all as seen in the window banner`: `mark_all_seen` throws `"Couldn't save. Try again."` → `findByRole("alert")` has that text.
  - App, new `shows the side-branch hint once`: two side-branch repos with `main_behind: 2` → `getAllByText("Update main refreshes main in the background. Your files stay as they are.")` has length 1.
  - App, new `subline says when it last checked`: `newsState()` → `getByText("Checked 2 min ago")`, `queryByText(/Since you last looked/)` null.
  - App, new `All quiet comes before the On a branch group`: the "keeps repos with something to act on visible…" state → `getByText(/Nothing new in your 2 repos/)` precedes heading `/^On a branch/` (`compareDocumentPosition`).
  - App, new `a news repo on a side branch sits in News`: news repo with `on_main: false, branch: "feature-x", main_behind: 2` → within the News section: "Update main" button and `getByText("feature-x")`.
  - App, new `a row with a stash alert stays out of Up to date until closed`: `get_app_state` first returns one news repo (`unsaved: true`), later (after `update_repo` ran) the same repo with `new_count: 0, behind: 0, feed: null`; `preview_update` → set-aside plan; `update_repo` → `edits_in_stash`. Click Update, then "Set aside and update" → the stash text is inside the section of heading `/^On main/`, and there is no Up to date accordion. Click "Close" → accordion `/Up to date 1 repo/` shows, heading `/^On main/` is gone.
  - App, "keeps errors in their own section…" becomes `puts Can't check first`: heading level 2 `/^Can't check/` precedes heading level 2 `/^News/`; "docs" is an `h3` after the Can't check heading; `getAllByText("Can't check")` has length 1 (the label, no badge).
  - App, "shows only errors as the headline when there is no news": also `getByRole("heading", { level: 2, name: /^Can't check/ })`, and no heading `/^News/`.
  - Footer: delete the two Mark all as seen tests; the Add folder test renders with `onAddFolder` only and also asserts `getAllByRole("button")` has length 1.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/__tests__/App.test.ts src/lib/__tests__/Footer.test.ts`
Expected: the new and changed tests fail.

- [ ] **Step 3: Implement** the interfaces above in `App.svelte` and `Footer.svelte`.

- [ ] **Step 4: Run all checks**

Run: `cd app && npm test && npm run check`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add app/src/App.svelte app/src/lib/Footer.svelte app/src/__tests__/App.test.ts app/src/lib/__tests__/Footer.test.ts
git commit -m "feat(ui): group repos by what they need"
```

### Task 7: Open and closed rules — complex

Label reason: state and event logic.

Load `impeccable` and read craft-floor.md before editing.

**Files:**
- Modify: `app/src/App.svelte`
- Test: `app/src/__tests__/App.test.ts`

**Interfaces:**
- Consumes: RepoCard `expanded` / `onToggle` (Task 2); event `"window-shown"` (Task 1).
- Produces in App:
  - `let openPaths = new Set<string>()`; every RepoCard gets `expanded={openPaths.has(r.path)}` and `onToggle={() => toggleRow(r.path)}`.
  - `function toggleRow(path: string)`: adds or removes, reassigning the set.
  - `let prevNews = new Set<string>()` and `function syncNews(news: RepoCardView[])`, run as `$: syncNews(newsRepos)`: the paths in `news` that are not in `prevNews` are "entering"; if `news.length <= 2`, add them to `openPaths`; then `prevNews` = the current paths. It never removes anything. First load is the same case (everything enters).
  - `function resetOpen()`: `openPaths` = all news paths when `newsRepos.length <= 2`, else empty; `prevNews` = the current news paths. `showUpToDate` is left as it is.
  - `listen("window-shown", resetOpen)` next to the other listeners, unlistened on destroy.
  - The `focus-repo` handler also adds the path to `openPaths` (before setting `highlightPath`). A row that can't open just ignores `expanded`.

- [ ] **Step 1: Write and change the tests first** (helper in the file: `news(n)` returns a state with `n` news repos `n0…`, each with `makeFeed()`; use `{ shouldMockEvents: true }` where events are emitted)
  - `opens news rows when 2 or fewer have news`: `news(2)` → both toggles `aria-expanded="true"`. `news(3)` → all `"false"`.
  - `a row the user opened stays open on state-updated`: `news(3)`, click toggle of `n0` → `"true"`; `emit("state-updated", news(3))` → still `"true"`.
  - `a closed row stays closed on state-updated when 2 or fewer have news` (Review Focus 2): `news(2)`, click `n0` → `"false"`; emit `news(2)` → `n0` still `"false"`, `n1` `"true"`.
  - `a repo entering News opens only while News has 2 or fewer`: start `news(1)` (open); emit `news(2)` → `n1` `"true"`; emit `news(3)` → `n2` `"false"`, `n0` and `n1` still `"true"`.
  - `a row that leaves News keeps its open state` (Review Focus 1): `news(3)`, open `n0`; emit a state where `n0` has `new_count: 0, behind: 2` and keeps its feed → `n0` is under heading `/^On main/` and `"true"`.
  - `window-shown re-applies the rule`: a state with 3 news repos, one on-main repo with `behind: 2, feed: makeFeed()` and one plain repo. Open `n0`, the on-main row and the accordion; `emit("window-shown")` → `n0` and the on-main row `"false"`, accordion still `aria-expanded="true"`.
  - "scrolls to and highlights that card" becomes `focus jump opens, scrolls and flashes the row`: `news(3)`, `emit("focus-repo", path of n1)` → scroll called, row has class `highlight`, toggle of `n1` `"true"`.
  - `focus jump to a row that can't open still scrolls and flashes` (Review Focus 3): `newsState()`, emit `focus-repo` for `${P}/docs` → scroll called, row has `highlight`, the row has no `aria-expanded` element.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/__tests__/App.test.ts`
Expected: the new and changed tests fail.

- [ ] **Step 3: Implement** the interfaces above.

- [ ] **Step 4: Run all checks**

Run: `cd app && npm test && npm run check`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add app/src/App.svelte app/src/__tests__/App.test.ts
git commit -m "feat(ui): open news rows when there is little news"
```

### Task 8: Check the real window in an isolated dev instance — simple

No code unless a check fails. A fix that is only CSS stays in this task; a fix that changes logic becomes its own complex task with a test.

Load `impeccable` and read craft-floor.md before any fix.

**Files:** none committed, except fixes.

- [ ] **Step 1: Build fixtures in a temp folder** with a throwaway script kept in its own temp folder (not in the repo). `T=$(mktemp -d)`. For each of 16 repos: a bare remote `$T/remotes/<n>.git`, the user's clone `$T/repos/<n>` with one commit pushed, and a team clone `$T/team/<n>`. Make 6 side-branch repos (`git checkout -b <branch>` in the user's clone; one branch named `feature/a-very-long-branch-name-to-check-wrapping`), and give 4 of them 2 team commits on main before the app starts. Name one repo `a-very-long-repository-name-for-the-ellipsis-check`. Add one repo whose remote path is then deleted, so it lands in Can't check with a long reason. Settings file `$T/config/repobell/settings.json`: `{"folders":["$T/repos"],"setup_done":true,"start_at_login":false,"check_minutes":5}`.

- [ ] **Step 2: Start the dev instance isolated from the user's app**

Run: `mkdir -p $T/run && chmod 700 $T/run && cd app && XDG_CONFIG_HOME=$T/config XDG_STATE_HOME=$T/state XDG_RUNTIME_DIR=$T/run npm run tauri dev` (in the background).
`XDG_RUNTIME_DIR` must be isolated: `tray_icon_dir()` deletes `$XDG_RUNTIME_DIR/repobell-tray`, which holds the running app's tray icons. `DBUS_SESSION_BUS_ADDRESS` is already set to a full path, so the tray still reaches the session bus. If the tray or window doesn't appear, stop and report; don't fall back to the real dirs.

- [ ] **Step 3: Make news.** After the first check, push 2 to 6 team commits to main in 3 repos (authors with `git -c user.name=Sara -c user.email=s@x commit`, two authors in one repo), then use "Check now".

- [ ] **Step 4: Check the Goals with a screenshot on display `:1`** at 420×640:
  - All 3 news rows and at least 4 On a branch rows are visible without scrolling; each number shows once per row.
  - The long repo name ends in an ellipsis with a hover; the long branch name and the error reason wrap and are fully readable.
  - A click on line 2 of a news row toggles it; a click on Update, a feed title or inside a preview doesn't.
  - Keyboard: Tab order is toggle, action, flags, panel controls; Enter and Space toggle; Escape closes the preview and focus goes back to Update.
  - With 3 news repos: open one row, close the window (it hides), then click another repo's line in the dev instance's tray menu → the first row is closed and the jumped-to row is open and flashed. This proves `window-shown` arrives before `focus-repo`.
  - With the window already showing, click a tray repo line → no row closes.

- [ ] **Step 5: Stop the dev instance and delete `$T`.** Report each check with its result and the screenshot paths.

### Task 9: Docs — simple

Run all new text through `humanizer`.

**Files:**
- Modify: `docs/decisions.md`, `DESIGN.md`, `ROADMAP.md`

- [ ] **Step 1: `docs/decisions.md`**: add the spec's "Main window layout" section (from "Decision text for docs/decisions.md") as written, before "## Design reviews".

- [ ] **Step 2: `DESIGN.md`**: change the Overview line that says it describes the card list to say it describes grouped rows. Add to Layout and Components: group lists (white, 10px radius, Hairline border and dividers, rows about 9px 12px), the section label (12px 700 uppercase Soft Ink, count in Muted Ink), row states (hover #fbfbf9, inset blue flash, inset red ring), 32px row buttons (an exception to the 36px minimum, covered by the whole-row hit area), and Update main as a Signal Blue text button with Blue Wash hover. Remove the compact card variant and the per-card conflict border. In the front matter replace `repo-card` with `repo-row` and add `button-text-blue`.

- [ ] **Step 3: `ROADMAP.md`**: add a "Main window redesign" item under Fixes (or a new short section) with the design, plan and build ticked, and set "Current step" to the build being done and waiting for the merge and hand test.

- [ ] **Step 4: Commit**

```bash
git add docs/decisions.md DESIGN.md ROADMAP.md
git commit -m "docs: record the grouped rows layout"
```
