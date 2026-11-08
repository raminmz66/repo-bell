# RepoBell Phase 3: Complete v1 App Specification

## 1. Overview & Architecture

RepoBell is an ambient desktop tray application for Linux (Ubuntu 22.04+) built with Tauri v2 (Rust backend) and Svelte (Vite frontend). RepoBell periodically checks watched Git repositories, detects unseen changes on the main branch, updates the system tray icon and menu, and provides a focused main window for reviewing changes and executing safe, reversible updates.

## 2. Data Models & File Contracts

### 2.1 Settings (`~/.config/repobell/settings.json`)
```json
{
  "folders": ["/home/user/projects"],
  "ignored": ["/home/user/projects/archived-repo"],
  "muted": ["/home/user/projects/noisy-repo"],
  "editor": "code",
  "check_minutes": 5,
  "start_at_login": true
}
```

### 2.2 Seen State (`~/.local/state/repobell/seen.json`)
```json
{
  "/home/user/projects/web-app": "9a3f2b1c...",
  "/home/user/projects/design-system": "7c1d4e0a..."
}
```

### 2.3 IPC Commands & Data Contracts

#### Query / State
- `get_app_state() -> AppStateView`:
  ```rust
  pub struct AppStateView {
      pub repos: Vec<RepoCardView>,
      pub settings: Settings,
      pub last_checked: Option<u64>, // epoch seconds
      pub offline: bool,
      pub first_run: bool,
  }
  ```
- `RepoCardView`:
  ```rust
  pub struct RepoCardView {
      pub path: PathBuf,
      pub name: String,
      pub folder: PathBuf,
      pub error: Option<String>,
      pub behind: u32,
      pub ahead: u32,
      pub unsaved: bool,
      pub muted: bool,
      pub feed: Option<FeedSummary>,
      pub undo_head: Option<String>,
  }
  ```
- `FeedSummary`:
  ```rust
  pub struct FeedSummary {
      pub total_changes: usize,
      pub author_summary: String, // e.g. "Sara added 4 changes to the main version"
      pub newest_time: Option<u64>,
      pub changes: Vec<ChangeItem>,
      pub files_changed: Vec<String>,
  }
  ```

#### Actions
- `check_now() -> AppStateView`: Triggers immediate fetch & recalculation of all watched repos.
- `mark_all_seen() -> ()`: Records the current remote main SHA for all watched repos in `seen.json`, updates tray to quiet (`bell.png`), and refreshes state.
- `update_repo(path: PathBuf, mode: UpdateMode) -> UpdateResultView`:
  `UpdateMode`: `"safe" | "set_aside" | "combine"`
  Calls `repobell_core::update`. Returns success (with undo point) or conflict (with list of conflicting files).
- `undo_repo(path: PathBuf, undo_head: String) -> UndoResultView`:
  Calls `repobell_core::undo`.
- `open_editor(file_path: PathBuf) -> Result<(), String>`:
  Opens conflicting file using configured editor or `xdg-open`.
- `save_settings(settings: Settings) -> ()`:
  Persists updated settings and updates background timer / autostart.
- `pick_folder() -> Option<PathBuf>`:
  Invokes native directory picker (`zenity --file-selection --directory`).
- `discover_suggested_folders() -> Vec<SuggestedFolder>`:
  Checks `~/projects`, `~/code`, `~/work`, `~/Documents` for first-run setup.
- `set_start_at_login(enabled: bool) -> ()`:
  Creates or deletes `~/.config/autostart/repobell.desktop`.

## 3. Background Check & Tray Icon Lifecycle

1. **Check Loop**:
   - Runs periodically based on `settings.check_minutes` (default 5 min).
   - Can be triggered on-demand via IPC (`check_now`) or tray menu item ("Check now").
   - Fetches `origin/main` sequentially for each watched repo using `repobell_core::fetch_main`.
   - Computes status & feed against `seen.json`.
   - If network failure affects all repos, sets `offline = true` and doubles interval with exponential backoff up to 60 minutes.

2. **Tray States**:
   - `bell.png`: All watched repos are up-to-date with 0 unseen changes.
   - `bell-dot.png`: At least 1 unmuted repo has unseen commits on remote main.
   - `bell-off.png`: Offline or all checks failed.

3. **Tray Menu Structure**:
   - Headline: `"X repos have news"` / `"All quiet"` / `"Can't check your repos"` (disabled item).
   - Separator.
   - Up to 5 repo items:
     - Label: e.g. `web-app — Behind by 4`
     - Clicking opens main window and focuses that repo.
   - If more than 5: `"and N more..."` (disabled item).
   - Separator.
   - `"Open RepoBell"`
   - `"Check now"`
   - `"Mark all as seen"`
   - Separator.
   - `"Quit"`

## 4. Notifications Engine

- Sent once per check cycle when new incoming teammate commits are detected.
- Single repo: `"[repo-name]: [N] new changes"`, Body: `"[Author] added [N] changes to the main version. [Change title]..."`.
- Multiple repos: `"RepoBell: [N] repos have news"`, Body: `"New changes were pushed to [list of repo names]."`.
- Never sent for user's own commits (`mine: true`).
- Muted repos send zero desktop notifications.
- Sent via native `/usr/bin/notify-send` with RepoBell icon.

## 5. UI Architecture (Svelte)

1. **Views**:
   - `WelcomeView`: 3-step first-run wizard (Find repos -> Select repos -> Tray & autostart).
   - `MainView`:
     - Header: Check now button, title, settings button, close button.
     - Headline & "Since you last looked" timestamp.
     - Active repo cards with:
       - Repo name + Update button (Safe, Unsaved changes confirmation, Combine confirmation, or Conflict alert).
       - Badges: `Behind by X`, `Has unsaved changes`, `Ahead by X`, `Can't check`.
       - Commit summaries + bullet items + expandable "N files changed".
       - Post-update Undo banner with live disable if files modified.
     - "Up to date" collapsible accordion.
     - Footer: "Add folder" button and "Mark all as seen" button.
     - Offline banner when all checks fail.
   - `SettingsView`:
     - Header with Back button and close button.
     - Watched folders with repos listed, notify toggle switch, and "Stop watching" action.
     - "Add folder" button.
     - Editor select (auto-detected VS Code, JetBrains, Sublime, Text Editor, System default).
     - Check interval select (1m, 5m, 15m, 30m, 60m).
     - Start at login toggle.

## 6. Phase 4: Release & Packaging Specification

### 6.1 Feed Grouping by Person and Day
- **Data structure**:
  ```typescript
  export interface AuthorDayGroup {
    author: string;
    mine: boolean;
    changes: ChangeView[];
  }

  export interface DayGroup {
    dayKey: string;      // YYYY-MM-DD
    dayLabel: string;    // "Today", "Yesterday", or "MMM D, YYYY"
    authors: AuthorDayGroup[];
  }
  ```
- **Behavior**:
  - Changes are grouped first by date (local timezone), then by author.
  - Days are ordered newest to oldest.
  - Within each day, authors are listed in order of their latest commit on that day.
  - Commits under each author display commit message/title and timestamp/hash.
  - Display is styled compactly matching the Ubuntu desktop design system.
  - Unit tests verify multiple authors, multiple dates, boundary dates, and empty inputs.

### 6.2 Linux Packaging (.deb & AppImage)
- **Tauri v2 Bundler**:
  - Target `.deb`: generated via `tauri build -b deb`.
  - Deb metadata: Category `DeveloperTool`, description, icon set, runtime library dependencies:
    - `libwebkit2gtk-4.1-0`
    - `libayatana-appindicator3-1`
    - `libxdo3`
  - Verified with `dpkg-deb --info` and `dpkg-deb --contents`.
  - Target `appimage`: generated via `tauri build -b appimage` when `appimagetool` is present.

### 6.3 Contributing Guide (`CONTRIBUTING.md`)
- Requirements & system libraries for Ubuntu.
- Rust and Node toolchains.
- Dev server and test execution (`cargo test`, `npm run build`).
- Architecture breakdown (`crates/repobell_core` vs `app/src-tauri` vs `app/src`).
- Conventional Commits and PR process.

### 6.4 Registry & Name Verification
- Name availability check for `repobell` / `repo-bell` on npm, crates.io, GitHub, and domain registries.
- Findings recorded in `docs/name-check.md` and referenced in `docs/decisions.md`.
