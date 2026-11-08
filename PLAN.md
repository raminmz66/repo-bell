# RepoBell Phase 3: Complete v1 App Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement full v1 RepoBell app: background checks, dynamic tray states & menu, notification engine, complete main window with feed/update/undo, settings management, and first-run wizard.

**Architecture:** Tauri v2 Rust backend with background worker thread communicating with tray and frontend webview via IPC commands and events. Svelte frontend with modular views (Main, Settings, Welcome) and responsive state management.

**Tech Stack:** Rust (Tauri v2, serde, repobell-core), Svelte 5, TypeScript, Vite.

**Spec:** [SPEC.md](SPEC.md)

## Global Constraints
- Target platform: Ubuntu 22.04+ (GNOME / X11).
- Non-blocking Git execution off the main thread.
- Atomic config saves (`settings.json` and `seen.json`).
- UI styling faithful to `docs/mockups/project/` HTML mockups with Ubuntu font and clean neutral palette.
- Rigorous verification before marking tasks complete.

---

### Task 1: Background Engine & Dynamic Tray Controller (Rust)
**Files:**
- Modify: `app/src-tauri/src/store.rs`
- Create: `app/src-tauri/src/watcher.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Test: `app/src-tauri/tests/watcher.rs`

**Interfaces:**
- Consumes: `repobell_core::{discover, status, fetch_main, feed, classify}`
- Produces: `AppStateView`, `check_now`, `mark_all_seen`, dynamic tray icon/menu updates.

- [ ] **Step 1: Write tests for watcher logic and feed aggregation**
- [ ] **Step 2: Implement watcher engine, background interval loop, and tray update handler**
- [ ] **Step 3: Wire tray menu dynamic items (headline, top 5 repos, actions) and left-click behavior**
- [ ] **Step 4: Verify with `cargo test` and interactive run**
- [ ] **Step 5: Commit** (`feat: add background check loop and dynamic tray controller`)

---

### Task 2: Core Update & Undo IPC Endpoints and File Opening
**Files:**
- Create: `app/src-tauri/src/actions.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Test: `app/src-tauri/tests/actions.rs`

**Interfaces:**
- Consumes: `repobell_core::update::{plan, update, undo, can_undo}`
- Produces: Tauri IPC commands: `update_repo`, `undo_repo`, `open_editor`, `pick_folder`, `save_settings`, `get_app_state`.

- [ ] **Step 1: Implement action handlers in `actions.rs`**
- [ ] **Step 2: Add editor discovery and opener (VS Code, JetBrains, Sublime, Text Editor, xdg-open)**
- [ ] **Step 3: Register all IPC commands in Tauri builder**
- [ ] **Step 4: Run unit and integration tests**
- [ ] **Step 5: Commit** (`feat: add update, undo, and editor IPC handlers`)

---

### Task 3: Notification Engine & Login Autostart
**Files:**
- Create: `app/src-tauri/src/notify.rs`
- Create: `app/src-tauri/src/autostart.rs`
- Modify: `app/src-tauri/src/watcher.rs`
- Modify: `app/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `notify-send`, `~/.config/autostart/repobell.desktop`
- Produces: Teammate commit notifications per check cycle; autostart toggling.

- [ ] **Step 1: Implement notify logic respecting `muted` repos and teammate commit filtering**
- [ ] **Step 2: Implement autostart desktop entry generator**
- [ ] **Step 3: Verify notification triggers and autostart file generation**
- [ ] **Step 4: Commit** (`feat: add desktop notification and autostart support`)

---

### Task 4: Svelte UI Implementation — Main Window, Repo Cards, Update & Undo
**Files:**
- Create: `app/src/lib/types.ts`
- Create: `app/src/lib/RepoCard.svelte`
- Create: `app/src/lib/Header.svelte`
- Create: `app/src/lib/Footer.svelte`
- Modify: `app/src/App.svelte`

**Interfaces:**
- Consumes: Tauri IPC `get_app_state`, `check_now`, `mark_all_seen`, `update_repo`, `undo_repo`, `open_editor`
- Produces: Full UI matching `Main.dc.html`, `UpdateStates.dc.html`, and `EmptyStates.dc.html`.

- [ ] **Step 1: Create TypeScript types and mock/store bindings**
- [ ] **Step 2: Implement `RepoCard.svelte` with badges, commit summaries, changed files, and Update/Undo modal states**
- [ ] **Step 3: Implement Main window layout with Header, Footer, and "Up to date" accordion**
- [ ] **Step 4: Verify with `npm run build` and UI screenshot testing**
- [ ] **Step 5: Commit** (`feat: implement main window repo cards, feed, and update/undo flows`)

---

### Task 5: Svelte UI Implementation — Settings Screen & First-Run Wizard
**Files:**
- Create: `app/src/lib/SettingsView.svelte`
- Create: `app/src/lib/WelcomeView.svelte`
- Modify: `app/src/App.svelte`

**Interfaces:**
- Consumes: `save_settings`, `pick_folder`, `discover_suggested_folders`, `set_start_at_login`
- Produces: Screens matching `Settings.dc.html` and `Welcome.dc.html`.

- [ ] **Step 1: Implement `SettingsView.svelte` with folder management, notify switches, editor select, interval select**
- [ ] **Step 2: Implement `WelcomeView.svelte` 3-step wizard with auto-suggested folders and tray check**
- [ ] **Step 3: Integrate navigation between Welcome, Main, and Settings in `App.svelte`**
- [ ] **Step 4: Verify with `npm run build` and UI screenshot testing**
- [ ] **Step 5: Commit** (`feat: implement settings view and first-run welcome wizard`)

---

### Task 6: End-to-End Integration, Verification & Cleanup
- [ ] **Step 1: Run all backend tests (`cargo test`) and frontend typechecks (`npm run build`)**
- [ ] **Step 2: Run the app live, take screenshots of Main, Settings, and Welcome states**
- [ ] **Step 3: Update `ROADMAP.md` checking off Phase 3 items**
- [ ] **Step 4: Merge `feat/phase3-v1-app` to `main`, push to remote, and delete feature branch**
