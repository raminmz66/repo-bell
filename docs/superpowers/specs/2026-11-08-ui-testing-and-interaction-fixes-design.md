# UI Testing & Interaction Fixes Design

## 1. Intent & Context

During manual UI testing of RepoBell, two critical bugs were uncovered:
1. **Window Close (X Button)**: Clicking the "X" button in the header does not close or hide the window to keep RepoBell running in the background.
2. **Settings Switch Reversion**: Toggling off watched repositories in the Settings view causes them to revert back to active ("Watched · Alerts on") after a few seconds.
3. **Card Interaction Latency/Clarity**: Clicking "Update" on a collapsed repo card with pending unsaved or combine states sets confirmation flags internally, but because the drawer is collapsed, the user sees no confirmation prompt, making the button appear unresponsive.

This design addresses these interaction defects and establishes a comprehensive automated UI test suite covering every button, click handler, and screen state across the entire application.

---

## 2. Goals, Non-Goals, & Anti-Goals

### Goals
- **Reliable Background Hiding**: Ensure clicking the "X" button immediately hides the window to the tray while keeping RepoBell running in the background.
- **Race-Condition-Proof Settings**: Prevent in-flight background checks from overwriting user settings and ensure that changes to watched/muted repos are synchronized immediately and permanently.
- **Visible Confirmation Prompts**: Auto-expand collapsed repo cards when an update requires user confirmation (unsaved files or incoming branch merge).
- **100% Button & Screen UI Test Coverage**: Implement automated unit/component tests using `@testing-library/svelte` and Vitest to click and verify every button, switch, link, and view transition.
- **Backend Verification**: Validate settings persistence, ignore list filtering, and engine thread safety with native Rust integration tests.

### Non-Goals
- Adding heavyweight external browser drivers (e.g. `tauri-driver`, Selenium) requiring a virtual display server in CI.
- Redesigning the visual layout or modifying UI theme colors.

### Anti-Goals
- Swallowing IPC errors silently without logging or fallbacks.
- Blocking the UI thread during background Git operations.

---

## 3. Evaluated Approaches

- **Approach 1: Fast Component & UI Test Suite Only**: Vitest + Testing Library with mocked Tauri IPC. Fast and deterministic, but does not verify native Rust engine behavior.
- **Approach 2: Hybrid Automated UI Suite + Native Engine Verification (Chosen)**: Full Vitest UI button testing across all screens combined with backend IPC fixes (`hide_window`), synchronization safeguards, and Rust integration tests.
- **Approach 3: Live End-to-End Desktop Automation**: Native WebDriver testing via `tauri-driver`. Rejected due to heavy setup and flakiness across environments.

---

## 4. Technical Architecture & Implementation Details

### 4.1 Window Dismissal (`hide_window`)
- Add `core:window:allow-hide` and `core:window:allow-close` to `app/src-tauri/capabilities/default.json`.
- Add a dedicated Tauri command `hide_window` in `app/src-tauri/src/actions.rs`:
  ```rust
  #[tauri::command]
  pub async fn hide_window(window: tauri::WebviewWindow) -> Result<(), String> {
      window.hide().map_err(|e| e.to_string())
  }
  ```
- In `Header.svelte`, call `hide_window` via `invoke("hide_window")`, falling back to `getCurrentWebviewWindow().hide()`.

### 4.2 Settings State Synchronization & Race Condition Fix
1. **Background Check Re-synchronization**:
   - In `actions::save_settings` and `lib::run`, when an asynchronous Git check finishes:
     ```rust
     let mut state = bg_engine.lock().await;
     // Re-apply the current (latest) settings to the computed repos
     for r in &mut repos {
         r.ignored = state.settings.ignored.contains(&r.path);
         r.muted = state.settings.muted.contains(&r.path);
     }
     state.repos = repos;
     state.offline = offline;
     state.last_checked = Some(watcher::now_seconds());
     ```
   - This guarantees that even if a background check started before the user toggled a switch, the newly emitted `state-updated` payload preserves the user's latest `ignored` and `muted` choices.
2. **Frontend Settings View State Locking**:
   - In `SettingsView.svelte`, ensure `toggleWatch` and `toggleMute` immediately update the local reactive state and queue persistence cleanly without being clobbered by older state updates.

### 4.3 Collapsed Card Update Prompt
- In `RepoCard.svelte`, when `handleUpdate()` triggers `confirmingUnsaved = true` or `confirmingCombine = true`, set `isExpanded = true` so the prompt box is immediately revealed to the user.

### 4.4 Automated UI Button Test Suite
- Add `@testing-library/svelte` and configure Vitest with `jsdom`.
- Test suites:
  1. `WelcomeView.test.ts`: Test Step 1 button ("Find my repos"), Step 2 repo checkboxes, "Add another folder", "Watch N repos" button, Step 3 autostart checkbox, and "Done" button.
  2. `Header.test.ts`: Test Check now button, Settings button, Back button, and Close window ("X") button.
  3. `Footer.test.ts`: Test "Add folder" button and "Mark all as seen" button.
  4. `RepoCard.test.ts`: Test drawer expand/collapse, Update button, Stash prompt ("Cancel", "Set aside and update"), Combine prompt ("Cancel", "Update and combine"), Conflict file list ("Open in editor", "Close"), Undo banner ("Undo"), Files changed toggle, and file links.
  5. `SettingsView.test.ts`: Test Back button, Remove folder button, Bell mute/unmute button, Watch switch toggle, Add folder button, Editor dropdown, Check interval dropdown, and Start at login switch.
  6. `App.test.ts`: Test root navigation between Welcome, Main, and Settings, offline "Try again", empty "Add folder", and Up-to-date accordion toggle.
