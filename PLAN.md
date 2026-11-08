# UI Testing & Button Interaction Fixes Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the window close ("X") button not hiding the app to the background, fix watched repositories reverting to active after toggling in Settings, auto-reveal confirmation prompts on collapsed cards, and implement a complete automated UI button test suite covering every screen and interaction.

**Architecture:** Tauri v2 capability configuration and IPC commands (`hide_window`), Rust engine synchronization in `watcher.rs` and `actions.rs`, Svelte 5 component interaction refinements, and Vitest + Testing Library DOM unit/integration test suites.

**Spec:** [docs/superpowers/specs/2026-11-08-ui-testing-and-interaction-fixes-design.md](docs/superpowers/specs/2026-11-08-ui-testing-and-interaction-fixes-design.md)

---

### Task 1: Window Hiding Backend Command & Capability
**Files:**
- Modify: `app/src-tauri/capabilities/default.json`
- Modify: `app/src-tauri/src/actions.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Modify: `app/src/lib/Header.svelte`

- [x] **Step 1: Add window permissions to `default.json`** (`core:window:allow-hide`, `core:window:allow-close`)
- [x] **Step 2: Implement `hide_window` command in `actions.rs` and register in `generate_handler!`**
- [x] **Step 3: Update `Header.svelte` to invoke `hide_window` with reliable fallback**
- [x] **Step 4: Verify Rust compilation (`cargo check -p repobell`)**
- [x] **Step 5: Commit** (`fix(window): enable window hiding to system tray on close button click`)

---

### Task 2: Background Watcher & Settings State Synchronization
**Files:**
- Modify: `app/src-tauri/src/actions.rs`
- Modify: `app/src-tauri/src/lib.rs`
- Modify: `app/src-tauri/src/watcher.rs`
- Create: `app/src-tauri/tests/settings_sync.rs`

- [x] **Step 1: Write integration test in `settings_sync.rs` asserting settings stability during background updates**
- [x] **Step 2: Synchronize in-flight background check results with latest engine settings in `actions.rs` and `lib.rs`**
- [x] **Step 3: Run `cargo test --test settings_sync` and verify all tests pass**
- [x] **Step 4: Commit** (`fix(settings): ensure background checks preserve latest ignored and muted repo settings`)

---

### Task 3: RepoCard & SettingsView Interaction Polish
**Files:**
- Modify: `app/src/lib/RepoCard.svelte`
- Modify: `app/src/lib/SettingsView.svelte`

- [x] **Step 1: In `RepoCard.svelte`, ensure `isExpanded = true` triggers when confirmation or conflict states activate**
- [x] **Step 2: In `SettingsView.svelte`, ensure reactive toggle operations queue cleanly and prevent race conditions**
- [x] **Step 3: Verify TypeScript and Svelte build (`npm --prefix app run build`)**
- [x] **Step 4: Commit** (`fix(ui): reveal confirmation prompts on collapsed cards and smooth settings toggling`)

---

### Task 4: Setup Testing Library Environment for Vitest
**Files:**
- Modify: `app/package.json`
- Modify: `app/vite.config.ts`

- [x] **Step 1: Install `@testing-library/svelte`, `@testing-library/jest-dom`, and `jsdom`**
- [x] **Step 2: Configure Vitest in `vite.config.ts` with `environment: 'jsdom'` and setup files**
- [x] **Step 3: Verify basic Vitest test runs cleanly**
- [x] **Step 4: Commit** (`chore(test): configure testing-library and jsdom for Svelte UI component tests`)

---

### Task 5: Comprehensive Automated UI Button Tests
**Files:**
- Create: `app/src/lib/__tests__/Header.test.ts`
- Create: `app/src/lib/__tests__/Footer.test.ts`
- Create: `app/src/lib/__tests__/WelcomeView.test.ts`
- Create: `app/src/lib/__tests__/RepoCard.test.ts`
- Create: `app/src/lib/__tests__/SettingsView.test.ts`
- Create: `app/src/__tests__/App.test.ts`

- [x] **Step 1: Implement `Header.test.ts` (Check now, Settings, Back, Close window buttons)**
- [x] **Step 2: Implement `Footer.test.ts` (Add folder, Mark all as seen buttons)**
- [x] **Step 3: Implement `WelcomeView.test.ts` (Step 1 Find repos, Step 2 repo checkboxes, Add folder, Watch repos, Step 3 Login toggle, Done)**
- [x] **Step 4: Implement `RepoCard.test.ts` (Header toggle, Update, Set aside, Combine, Cancel, Open editor, Close conflict, Undo, Files toggle, File links)**
- [x] **Step 5: Implement `SettingsView.test.ts` (Back, Remove folder, Bell alert toggle, Watch switch, Add folder, Editor select, Interval select, Login switch)**
- [x] **Step 6: Implement `App.test.ts` (Navigation flows, Offline try again, No repos add folder, Up-to-date accordion)**
- [x] **Step 7: Run all frontend tests (`npm --prefix app test`) and ensure 100% pass**
- [x] **Step 8: Commit** (`test(ui): add comprehensive button and screen interaction tests`)

---

### Task 6: Final Verification, Integration & Merge
- [x] **Step 1: Run full verification suite (`cargo test --workspace`, `npm --prefix app test`, `npm --prefix app run build`)**
- [ ] **Step 2: Merge `fix/ui-button-interactions-and-tests` into `main`**
- [ ] **Step 3: Push `origin/main`**
- [ ] **Step 4: Delete local branch `fix/ui-button-interactions-and-tests`**
