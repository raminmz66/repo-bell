# Grouped rows design

Date: 2026-11-14. Status: draft, waiting for Ramin's approval.

Visual target: option "A Grouped rows" and Q3 option "2 Open when 2 or fewer" in the approved mockup `layout-options.html` (job 3589d629). Where this spec and the mockup differ, the spec wins (see "Where the mockup differs").

## Why

The main window is hard to scan. Measured at the real 420×640 window with 16 watched repos (3 with news, 6 on side branches, 7 up to date), only 1 of the 9 repos that need a look is visible before scrolling. Every news card opens its feed by itself, so one card fills the screen.

The same number shows up to 4 times ("6 new", "Behind by 6", "mojitiative added 6 changes", "mojitiative 2 changes"). Status sits in 4 different spots on a card. The feed nests day, border, author and bullet to show 3 titles. The side-branch hint ("Refreshes the main version in the background...") repeats on every side-branch card. There are 6 equal blue outlined buttons, and side-branch repos have no heading.

PRODUCT.md's first principle is "a glance first": the window must answer "what's new and what needs me" before any scrolling. The main user watches 10 to 30 repos.

## Goals

- With the 16-repo set above, all 3 news rows and at least 4 side-branch rows are visible at 420×640 without scrolling.
- Each fact has one fixed place in a row, and each number appears once.
- Repos are grouped by what they need: can't check, news, on a branch, up to date.
- Nothing a card can do or show today is lost.

## Non-goals

- New features or new backend data.
- Redesigning the in-row panels (update preview, conflict list, stash alert, Undo banners, notices, branch cleanup list). They move under the row as they are.
- Dark theme.
- Changes to the tray menu, Settings, the welcome wizard or notifications.

## Anti-goals

The redesign fails, even if it works, if any of these happen:

- An Undo banner, conflict list, stash alert or update preview is hidden in a closed group or lost when a row moves to another group.
- A row the user opened closes by itself while the window is open.
- Status text is cut off so the user can't read "Unsaved", a branch name or an error reason.
- The window needs Git words beyond today's short status labels.

## Approaches

Three layouts were mocked at real size with the 16 real repos.

- A. Grouped rows (picked). Every repo is one or two lines inside a group list. Fits all 3 news and 4 branch repos above the fold. Rows are denser, so the whole row is the click target and row buttons stay 32px tall. Reading titles takes a click (fixed by the Q3 rule below).
- B. Leaner cards. Today's cards, trimmed. Smallest change and no click to read titles, but at 5 or more news repos it scrolls again, so it doesn't scale to 30 repos.
- C. List, then repo page. Densest overview, but every action needs 2 clicks plus Back, the list shows no titles, coloured dots need a legend, and it is the largest build.

Ramin picked A on 2026-11-14.

Which news rows start open (Q3):

1. All closed. Simplest, but with 1 or 2 news repos you always click to read titles.
2. Open when 2 or fewer (picked). Little news reads at once; 3 or more stays an overview.
3. Newest open. One open row pushes the branch group below the fold at 3 or more news repos.

## What the user sees

### Top

- Header: unchanged.
- Headline row: the bell icon and headline stay. The subline becomes "Checked 4 min ago" ("Checking…" while a check runs). "Since you last looked" is dropped.

### Groups

The main list shows groups in this order. A group with no repos is not shown.

1. Can't check: repos with an error, then missing folders.
2. News: repos with `new_count > 0`.
3. On main (Resolved point 1). Repos on main that are not plain up to date: behind with no news (news was marked seen but not pulled), just updated with Undo still possible, or showing an alert panel (see "Rows with an alert stay in view").
4. On a branch: repos not on main.
5. Up to date: the existing collapsed accordion ("Up to date · 7 repos"), same rule as today (`on_main && behind === 0 && !can_undo`), except rows with an alert panel.

Each group (except the accordion) has a small uppercase label with its count, such as "NEWS 3", then one white bordered list (10px radius, 1px Hairline) with 1px Hairline dividers between rows. Groups are not separate cards.

- The News label has a text button on its right: "Mark all as seen". It calls `mark_all_seen`, as the footer button does today.
- Under the On a branch label, one muted line: "Update main refreshes main in the background. Your files stay as they are." It shows once, not per row.
- The accordion opens to the same kind of bordered row list.

A repo with news that is on a side branch goes in News (news wins). Its action is "Update main", and its branch name shows as a grey flag on line 2. (Claude's call.)

### Row

Line 1, left to right:

- A chevron, only when the row can open (it has a feed and no error).
- The repo name (14px, bold, cut with an ellipsis if too long; full name on hover).
- News rows: the "N new" badge, with today's hover text.
- On the right: news rows show the newest change time in short form ("30 min", "1 h", "3 d", "now"; hover "Newest change 30 min ago"), then the row's action button.

Line 2 is muted 13px text. The text comes first and the flags follow. Flags wrap onto a new line rather than cutting the text.

| Row | Line 2 text | Action (line 1, right) |
|---|---|---|
| News | Authors, then "· N files" (no files part when there are none) | Update (blue outline, 32px). Update main on a side branch. |
| On main, behind | "Behind by N" with today's hover | Update |
| On main, just updated | (no text, flags only) | none |
| On a branch | "branch-name · main N behind" or "branch-name · main up to date". Detached: "Not on a branch · ..." | Update main (quiet text button) only when main is behind |
| Up to date | (no text, flags only) | none |
| Can't check | The error reason, wrapped, never cut | Try again (outline) or Remove (secondary), as today, 32px |
| Missing folder | The message, then the full path on a third line (mono, muted, cut with hover) | Remove |

Authors: the distinct authors in `feed.changes`, newest first, "You" for the user's own changes. Up to 2 names, then "and N others" ("Sara, Amine and 1 other"). (Claude's call.) See Resolved point 2 for the 5-change limit.

Flags, in this order, each only when it applies:

- "Unsaved" (amber chip), hover "You edited files that aren't saved as a change yet".
- "N old branches" (grey chip). Still the button that opens the cleanup list, with today's hover and `aria-expanded`.
- "Ahead by N" (grey chip), only where today's chip shows it (on main, `ahead > 0`), with today's hover.

"Behind by N" no longer shows on news rows. The Update preview already says how many changes it downloads. The "Can't check" badge no longer shows on error rows, because the group label says it. The "Up to date" and "Main is up to date" texts move into the group label and line 2.

Update main gets the side-branch hint as its hover text, so a news row on a side branch still explains it. (Claude's call.)

Row states: hover on an openable row uses #fbfbf9 and a pointer cursor; an open row keeps #fbfbf9. The focus jump flash uses an inset 2px Signal Blue ring with #f4f8fe fill for 2.2s. While a conflict or stash alert shows, the row gets an inset 1.5px red (#c4372b) ring, replacing today's red card border. (Claude's call.)

### Open news row

Under line 2, full row width:

- A flat list of the latest 3 titles, newest first. No day labels, no author headers, no left border. Titles are shown exactly as written and wrap instead of being cut.
- When the 3 titles have more than one author, each starts with the author ("Sara: Fix login", "You: Bump version"). With one author, no prefix.
- Then one line: "N more · N files changed". "N more" is `total_changes` minus the titles shown and is left out at 0. "N files changed" is today's toggle button that opens the clickable file list. Left out with no files.

The "Alex added 2 changes to the main version" summary line is gone. Line 2 carries who.

### In-row panels

These appear under the row's two lines (and under the open feed), full row width, unchanged: the update preview and confirm prompt, the conflict list with Open in editor, the stash alert, both Undo banners, notices with Details, the old-branch cleanup list with Delete, and inline error text from a failed action. They show whether or not the row is open.

### Footer

Only "Add folder". The tray menu's "Mark all as seen" is unchanged.

## Open and closed rules (decided)

- The open set is owned by App, not by each row, so it survives a row moving between groups.
- When the window is shown, and at first load: if 2 or fewer repos have news, all news rows open; otherwise all news rows close. Rows outside News always close on this reset. The Up to date accordion keeps its state, as today. (Claude's call.) This replaces today's "a news card starts open" and "fresh news on a quiet card opens it" rules.
- Clicking a row toggles it. Only the user, the "window shown" reset and a focus jump change it.
- A background state update while the window is open never closes a row. A repo that enters News while the window is open starts open only if News then has 2 or fewer repos; rows already open stay open.
- A row that leaves News keeps its open state. If it can no longer open (no feed), it just shows closed.
- "Window shown" means the window went from hidden to visible. "Open RepoBell" or a tray repo line clicked while the window is already showing does not reset rows, so it can't close a row the user opened. (Claude's call, from the builder check.)
- A focus jump from a tray repo line always opens that repo's row (if it can open), opens the Up to date accordion when the repo is in it, and keeps today's scroll to center and 2.2s flash. Notifications have no click action on this machine today (`notify.rs`), so the tray is the only source.
- The "window shown" reset runs before the focus jump, so the jumped-to row stays open.

## Keyboard and screen reader

- Each group is a `<section>` with an `<h2>` label (visually the small uppercase label) and a `<ul>` of rows. Repo names become `<h3>`.
- An openable row has one real `<button>` holding the chevron, name and badge, with `aria-expanded` and `aria-controls` pointing to the open area. No `div role="button"`. A pseudo-element stretches its hit area over the row's two lines, so a click anywhere on lines 1 and 2 toggles it. The open feed and the panels are outside the hit area, so clicking a title or a preview never closes the row. (Claude's call, from the builder check.)
- The Update, Update main, Try again, Remove and "N old branches" buttons sit above that hit area and never toggle the row.
- Tab order follows the DOM: row toggle, action button, then flag buttons, then panel controls. Enter and Space toggle. Escape still closes the update preview and returns focus to Update.
- Rows that can't open have no toggle button and no pointer cursor; the name is plain text in the `<h3>`.
- Focus keeps the 2px Signal Blue outline. Chevron and flash transitions are off under reduced motion.

## Where every state renders

| State | Where |
|---|---|
| Update preview and confirm | Panel under the row |
| Conflict list, Open in editor | Panel under the row, row gets the red ring, row stays out of Up to date until closed |
| Stash alert with Details | Panel under the row, row gets the red ring, row stays out of Up to date until closed |
| Undo after update | Panel under the row (row sits in On main or News) |
| Undo after branch cleanup | Panel under the row, in whatever group the row is in |
| Notices, Details link | Panel under the row |
| Old-branch list, Delete | Opened by the "N old branches" flag, panel under the row |
| Repo error text and actions | Can't check group: reason on line 2, action on line 1 right, failed-action text under the row |
| Missing folder | Can't check group, after repo errors |
| Fresh state load error ("RepoBell can't start") | Unchanged |
| Window notice banner (failed Check now, Mark all as seen, Add folder) | Unchanged, above the headline |
| Offline | Unchanged |
| First check | Unchanged |
| No repos / all repos stopped | Unchanged |
| All quiet | Unchanged message and text. On main and On a branch groups and the accordion follow it, so the message sits above the On a branch group |
| Errors but no news | Headline as today (crossed bell, "1 repo can't be checked"), then the Can't check group |

## Component breakdown

- App.svelte: computes the five groups; renders them with one snippet for the label + list (label text, count, optional right-side action, optional hint). Owns the open set, the Q3 rule, the "window shown" listener and the focus jump. Builds the new subline. Drops the error `section-title` and passes `hasNews`/`onMarkAllSeen` to the News label instead of the footer.
- RepoCard.svelte: keep the file, its props and all its script logic (update, undo, cleanup, Update main, errors). Rewrite only the markup to the row layout, with `<li>` as the root and `data-repo-path` kept. Replace the internal `isExpanded`, `trackNews` and the expand part of `reveal()` with two props: `expanded` and `onToggle`. `reveal()` keeps scrolling. Remove the per-card hint, the "Can't check" badge, the "Up to date" text and the "Behind by" on news rows. Renaming the file is not needed.
- Row state across groups: RepoCard keeps its panel state (plan, notice, conflict files, stash alert). Each group is a separate keyed list, so a row that changes group is a new component and loses that state. The builder must keep panel state across a group move, for example a module-level map keyed by repo path that a row reads on mount. App also needs to know which rows show an alert (next point). A test must cover it.
- Rows with an alert stay in view: an Update that put unsaved edits aside and couldn't bring them back (`edits_in_stash` with `moved`) records no undo point (`actions.rs`, `update_repo`), so after the refresh the repo is behind 0 with `can_undo` false and would land in the closed Up to date list. A row showing the update preview, the conflict list, the stash alert, a warning or error notice, or a failed-action message therefore never goes in Up to date; it shows in On main until the panel is closed. Success and info notices don't count ("Already up to date." may go to Up to date). (Claude's call, from the builder check; found by reading the code, not in the running app.)
- FeedList.svelte: drop the summary line, the day and author grouping and the left border. Render the flat 3-title list, the author prefix rule, and the "N more · N files changed" line. Keep the file list and its open-in-editor handling.
- feedGrouping.ts and its test: delete. Remove the `DayGroup` and `AuthorDayGroup` types from `types.ts`. Nothing else uses them.
- FolderErrorCard.svelte: restyle as a row (`<li>`) for the Can't check list. Same logic.
- Footer.svelte: remove Mark all as seen and its props. Keep Add folder.
- util.ts: add a short time form (today's `timeAgo` without " ago", "just now" → "now"), or a flag on `timeAgo`. Builder's call.
- Backend: no new data. Authors come from `feed.changes`, files from `feed.files_changed`, time from `feed.newest_time`. One small change is needed for the "window shown" rule: see Resolved point 3.

Styling follows DESIGN.md tokens, light theme only: label 12px 700 uppercase in Soft Ink with the count in Muted Ink, row name 14px 700, line 2 13px, flags as chips at 12px with smaller padding, row padding about 9px 12px. Update main as a text button uses Signal Blue text, no border, Blue Wash on hover. Row buttons are 32px tall, below DESIGN.md's 36px minimum; the whole-row hit area makes up for it (decided).

## Where the mockup differs

- Mockup news line 2 sometimes shows a title ("siamoni · Add the release-notes skill"). This spec uses authors · files on every news row.
- Mockup prefixes "Logan:" on titles under a row whose line 2 names "mojitiative". This spec prefixes only when the shown titles have more than one author.
- Mockup shows "4 old" in one row. This spec uses "N old branches" everywhere.
- Mockup label says "Mark all seen". This spec uses "Mark all as seen", the same words as the tray. (Claude's call.)
- Mockup flags use a 5px radius. This spec uses 6px from the scale.

## Tests

The app uses Vitest with Testing Library. New tests:

App.test.ts
- Groups render in order Can't check, News, On main, On a branch, Up to date, each label with its count; empty groups are absent.
- "Mark all as seen" on the News label calls `mark_all_seen`; the footer has no such button.
- 2 news repos: both rows start open. 3 news repos: all start closed.
- Clicking a row toggles `aria-expanded`; a `state-updated` event afterwards does not close it.
- A repo entering News while the window is open: opens when News has 2 or fewer, stays closed at 3 or more; open rows stay open.
- The "window shown" event re-applies the rule (closes rows at 3 or more news repos, closes open non-news rows, leaves the accordion as it was).
- Focus jump opens a closed row with 3 or more news repos, scrolls and flashes it.
- The side-branch hint shows once with 2 side-branch rows.
- Subline reads "Checked 2 min ago" and has no "Since you last looked".
- All quiet with a side-branch repo: the message comes before the On a branch group.
- A news repo on a side branch sits in News with Update main and a branch flag.
- After an `edits_in_stash` update with `moved`, the row moves from News to On main (not Up to date) and the stash alert still shows; closing the alert lets the row go to Up to date.

RepoCard.test.ts
- News line 2: "Sara, Amine · 2 files"; "You" for own changes; "1 file"; no files part with none; 3 or more authors use "and N others".
- No "Behind by" on a news row; the badge keeps its hover.
- On-main behind row shows "Behind by 2" with its hover.
- Side-branch line 2: "feature-x · main 3 behind", "feature-x · main up to date", "Not on a branch · ...".
- Flags: "Unsaved" with hover, "N old branches" opens the list without toggling the row, "Ahead by 1" with hover.
- A click on line 2 toggles the row; a click on Update, on a title in the open feed or inside a panel does not.
- A row that can't open has no toggle button; no element has `role="button"`.
- Short time shows "5 min" with the full hover.
- Open feed: at most 3 titles, no day labels or author headers; author prefix only with more than one author; "5 more · 2 files changed"; no "more" when all fit.

Footer.test.ts
- Only Add folder renders and calls `onAddFolder`.

Existing tests that change:

- App.test.ts
  - "uses the backend headline and shows the news count..." : expect no "Behind by 4".
  - "keeps errors in their own section, not among the news": errors now come first, under an `h2` "Can't check"; repo names are `h3`; "Can't check" is the label, not a badge.
  - "shows only errors as the headline when there is no news": add the Can't check label check.
  - "tucks plain up-to-date repos into an accordion of compact rows": no `compact` class, no "Up to date" text in the row, row found by `data-repo-path` instead of `article`.
  - "keeps repos with something to act on visible...": "You're on feature-x" becomes "feature-x · main 2 behind".
  - "Mark all as seen calls mark_all_seen...": moves to the news tests, button found in the News label.
  - "scrolls to and highlights that card": also asserts the row opened.
- RepoCard.test.ts
  - "shows the news count as the badge and Behind as a quiet status next to Update": rewritten, no Behind on news rows.
  - "has no news badge when only behind": stays, Behind is on line 2.
  - "shows Ahead with hover text and Has unsaved changes": "Unsaved".
  - "renders an up-to-date repo as a compact row...": no "Up to date" text, no `compact` class.
  - "starts a news card expanded and a quiet one with own changes collapsed": replaced by the `expanded` prop test; the rule itself moves to App tests.
  - "uses a real disclosure button inside the heading...": `h3`; checks a title instead of the summary line.
  - "expands and scrolls into view when highlighted": only scroll and flash; opening moves to App tests.
  - "shows at most 3 titles and then 'and N more'": text becomes "5 more · 2 files changed"; tests pass `expanded: true`.
  - "groups by person when several people changed things": replaced by the prefix test.
  - side branch: "names the branch and offers Update main": line 2 text, no per-row hint. "shows a quiet note instead of the button...": "main up to date" on line 2. "says so when HEAD isn't on a branch": new text.
  - error card: "shows the reason, a Can't check badge and Try again": no badge.
  - Feed tests that read titles pass `expanded: true`.
- Footer.test.ts: the two Mark all as seen tests go; the Add folder test drops the extra props.
- feedGrouping.test.ts: deleted with `feedGrouping.ts`.
- `test-helpers.ts`: unchanged (`author_summary` stays in the type; the UI just doesn't show it).
- Unchanged: Header, SettingsView, WelcomeView, no-alert.

Before finishing, the builder checks the result at the real 420×640 window with a screenshot against the Goals.

## Out of scope

- Tray menu (including its "Mark all as seen"), Settings, the welcome wizard, notifications.
- Backend logic and data, except the one event in Resolved point 3.
- Dark theme.

## Left to the builder

- Exact spacing, chip padding and colors within DESIGN.md tokens.
- How panel state survives a group move, as long as the test passes.
- The short time helper's shape.
- Final wording of new hover texts. Text the user reads goes through the `humanizer` skill.

## Resolved points

Decided by Ramin on 2026-11-14.

1. Repos on main that are neither news nor plain go in their own "On main" group between News and On a branch. Line 2 keeps "Behind by N" (the only number in that row). The group is absent when empty.
2. Authors come from the newest 5 changes only (`SHOWN_CHANGES` in `watcher.rs`), so an author of an older change can be missing from line 2. Accepted; no backend field.
3. "When the window opens" gets a small backend change: `show_and_position_window` checks `is_visible()` before `show()` and emits `window-shown` only when the window was hidden. App listens to it. A focus event can't be used, because it also fires on Alt+Tab. It is emitted before `focus-repo` from the same thread; the builder confirms the order in the running app.
4. Today's window likely loses the stash alert after an update that moved but couldn't restore unsaved edits: the row moves to the closed Up to date list and remounts. Found by reading `actions.rs`, not tested in the running app. This design fixes it as a side effect ("Rows with an alert stay in view"); no separate fix.

## Decision text for docs/decisions.md

Add when built:

> ## Main window layout
>
> Decided 2026-11-14. Full design: `docs/superpowers/specs/2026-11-14-grouped-rows-design.md`.
>
> The main window shows repos as rows in groups, not as cards: Can't check, News, On main, On a branch, then the collapsed Up to date list. Each group is one bordered list with a small label and its count. A row has two lines: name, "N new" badge, time and action on the first; who changed things or the branch on the second, with small flags for unsaved changes, old branches and Ahead. Each number appears once, so "Behind by N" no longer shows on news rows; the Update preview says how many changes it downloads.
>
> When the window opens, news rows start open if 2 or fewer repos have news, otherwise closed. Clicking a row opens it. A background check never closes a row the user opened. A tray jump always opens that repo's row.
>
> "Mark all as seen" moved from the footer to the News label. The side-branch hint shows once under the On a branch label. Update main is a quiet text button, so a blue outline means the team has news for you.
>
> Measured before the change: 1 of 9 repos that needed a look fit above the fold at 420×640 with 16 repos. Layout B (leaner cards) scrolls again at 5 news repos, and C (list then repo page) needs 2 clicks plus Back per action.

## Docs to update when built

- `docs/decisions.md`: the section above.
- `DESIGN.md`: rows and group lists, 32px row buttons, the section label, Update main as a blue text button, and the overview line that says it describes the card list.
- `ROADMAP.md`: the new item and the current step.
