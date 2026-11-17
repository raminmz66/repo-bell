# About and Updates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Settings ends with an About section (version, update status, one button, notes, links); RepoBell checks GitHub for a signed new version at start and every 24 hours, and installs it after one click; the footer and the tray say when one is ready; the release workflow signs the packages and publishes `latest.json`.

**Architecture:** A new Rust module `updates.rs` owns `UpdateState` (inside `EngineState`), the check, the 24-hour timer and the install task, using `tauri-plugin-updater` from Rust only (the webview gets no updater permission). State goes out through the existing `publish` (window event and tray) as `AppStateView.update`. Repo actions take an `ActionGuard` so Install and a repo action never overlap. The window reads `state.update`: `AboutSection.svelte` at the end of Settings, a footer button, and an `open-about` event from the tray. `scripts/release-files.sh` builds the release folder and `latest.json`.

**Tech Stack:** Rust (Tauri 2.12, `tauri-plugin-updater` 2.14, tokio `Mutex`), Svelte 5 (legacy `export let` / `$:` syntax, as the files use today), Vitest + Testing Library + `@tauri-apps/api/mocks`, bash + `jq`, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-11-17-about-and-updates-design.md` (read it in full; it has the copy, the state table, the error mapping, the edge cases and the tests this plan refers to). Visual target: option B and the "Update states" row in `artifacts/about-and-updates.html` (git-ignored, in this worktree). Where the mockup differs from the spec, the spec wins.

## Global Constraints

- Work only in `/home/ramin/projects/repo-bell/.claude/worktrees/public-cleanup` (branch `about-and-updates`). Never touch the user's running RepoBell or its data (`~/.config/repobell`, `~/.local/state/repobell`, `$XDG_RUNTIME_DIR/repobell-tray`). Never run git against another checkout. Never push, tag or release.
- Never read or print `~/.tauri/repobell.key`.
- `.impeccable/config.json` has an unrelated change: never stage or commit it. Always `git add <explicit paths>`.
- Dependencies: only `tauri-plugin-updater = "2.14"` (default features) in `app/src-tauri/Cargo.toml`. No `tauri-plugin-process`, no npm packages, no capability entries.
- Public key for `plugins.updater.pubkey`, exactly: `dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDNDRDgzODJEMzIyODVFODcKUldTSFhpZ3lMVGpZUElDWlJzc0NCK0xGVXJqUWlIUVpZLzFmMDF6ZHVad0hmeTVYclpVMnpJVEsK`
- Endpoint: `https://github.com/raminmz66/repo-bell/releases/latest/download/latest.json`. Links: `https://github.com/raminmz66/repo-bell` and `https://github.com/raminmz66/repo-bell/issues/new`.
- `bundle.createUpdaterArtifacts` stays out of `tauri.conf.json` (only the release workflow turns it on with `--config`).
- 1 MB = 1,000,000 bytes, in Rust and in the window.
- Copy is fixed by the spec's state table, Footer and Tray sections ("…" is one character). Builders do not run text-check skills; the main session checks all text at the end.
- Blue is never used for the update dot or the download bar (One Blue Rule).
- Every UI task: load the `impeccable` skill and read `/home/ramin/projects/repo-bell/.claude/skills/impeccable/reference/craft-floor.md` before the first UI edit. Follow DESIGN.md tokens. Where craft-floor disagrees with the spec, the spec wins.
- Keep the files' existing Svelte style (`export let`, `$:`, `onclick=`). Light theme only.
- Test first. Write only the tests a task lists (ponytail mode).
- Rust checks, from the worktree root: `export PATH="$HOME/.cargo/bin:$PATH" && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`.
- Frontend checks: `cd app && npm run check && npm test` (`app/node_modules` is installed).
- If crates.io or GitHub is slow or unreachable, stop and report it; don't work around it.
- Commits: Conventional Commits, one short line, body ends with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

## Review Focus

1. A repo command binds its guard as `let _ = begin_action(…)`, which drops it at once, so Install can restart mid-Update. Task 3 binds `let _action` and greps for it.
2. Install pressed twice fast: the second call must be refused (status is no longer `Ready`), never start a second download. Test in Task 1 (`start_install`).
3. Release notes with `<b>` or `<script>` text show as plain text, never HTML. Test in Task 6.
4. After a tray or footer jump to About, opening Settings with the gear starts at the top again. Test in Task 7.
5. A `.sig` file that ends with a newline gives a `latest.json` signature with no trailing newline (the plugin base64-decodes it). Check in Task 8.

---

### Task 1: Updater plugin, `UpdateState` and its pure rules — complex

Label reason: security (signature config), adds state and branches, touches 6 files.

**Files:**
- Modify: `app/src-tauri/Cargo.toml`, `Cargo.lock`, `app/src-tauri/tauri.conf.json`, `app/src-tauri/src/lib.rs`, `app/src-tauri/src/watcher.rs`
- Create: `app/src-tauri/src/updates.rs` (with `mod tests`)

**Interfaces:**
- Produces in `updates.rs` (all `pub`):
  - `const MB: u64 = 1_000_000;`
  - `#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] enum UpdateStatus { #[default] Idle, Checking, UpToDate, Ready, Downloading, Installing }`
  - `#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] enum UpdateError { Offline, DownloadFailed, BadDownload, NotInstalled }`
  - `#[derive(Default)] struct UpdateState { current: String, can_update: bool, deb: bool, status: UpdateStatus, checked_at: Option<u64>, last_try: Option<(u64, bool)>, pending: Option<tauri_plugin_updater::Update>, downloaded: u64, total: Option<u64>, error: Option<UpdateError> }` (fields `pub`).
  - `#[derive(Clone, Debug, Serialize, Deserialize)] struct Available { version: String, notes: String }` and `#[derive(Clone, Debug, Serialize, Deserialize)] struct UpdateView { current, can_update, deb, status, checked_at, available: Option<Available>, downloaded, total, error, busy: bool }` matching the spec's `UpdateView` in `types.ts` (status and error serialize as the spec's strings, e.g. `up_to_date`, `download_failed`).
  - `UpdateState::for_bundle(current: String, bundle: Option<tauri::utils::config::BundleType>) -> Self`: `can_update` for `Deb` or `AppImage`, `deb` for `Deb`.
  - `UpdateState::view(&self, actions_running: u32) -> UpdateView`: `available` from `pending` (`version`, `body.unwrap_or_default()`), `busy = actions_running > 0`.
  - `UpdateState::begin_check(&mut self) -> Option<UpdateStatus>`: `None` unless `can_update` and status is `Idle` or `UpToDate`; else sets `Checking`, clears `error`, returns the old status.
  - `UpdateState::finish_check(&mut self, found: Result<Option<tauri_plugin_updater::Update>, tauri_plugin_updater::Error>, before: UpdateStatus, manual: bool, now: u64)`: `Ok(Some(u))` → `pending = Some(u)`, `Ready`, `checked_at = Some(now)`; `Ok(None)` → `UpToDate`, `checked_at = Some(now)`; `Err` → `status = before`, `error = manual.then_some(Offline)`. Always `last_try = Some((now, found.is_ok()))`.
  - `UpdateState::start_install(&mut self, actions_running: u32) -> Result<(), String>`: `Err("Wait for the repo action to finish.")` when `actions_running > 0`; `Err("There's no update to install.")` when status isn't `Ready`; else `Downloading`, `error = None`, `downloaded = 0`, `total = None`.
  - `UpdateState::action_allowed(&self) -> Result<(), String>`: `Err("RepoBell is installing an update and will restart in a moment.")` while `Downloading` or `Installing`.
  - `fn wait_for(setup_done: bool, u: &UpdateState, now: u64) -> std::time::Duration`: setup not done → 60 s; status not `Idle`/`UpToDate` → 60 s; `last_try` `None` → 0; `Some((t, true))` → `t + 24 h - now` (saturating); `Some((t, false))` → `t + 1 h - now`.
  - `fn error_for(e: &tauri_plugin_updater::Error, installing: bool) -> UpdateError`: one `match` per the spec's error mapping table.
  - `fn mb_changed(before: u64, after: u64) -> bool`: `before == 0 || before / MB != after / MB`.
- `watcher.rs`: `EngineState` gains `pub update: UpdateState` and `pub actions_running: u32` (`UpdateState::default()` and `0` in `new()`); `AppStateView` gains `pub update: UpdateView`, filled in `to_view` with `self.update.view(self.actions_running)`.
- `lib.rs`: `pub mod updates;`, `.plugin(tauri_plugin_updater::Builder::new().build())`, and in `setup` (before the scheduler spawn): `engine.state.blocking_lock().update = UpdateState::for_bundle(app.package_info().version.to_string(), tauri::utils::platform::bundle_type());`
- `tauri.conf.json`: top-level `"plugins": { "updater": { "pubkey": "<key from Global Constraints>", "endpoints": ["<endpoint>"], "requireSignedVersion": true } }`.

- [ ] **Step 1: Add the dependency and config.** `cargo add tauri-plugin-updater@2.14 -p repobell` (or edit `Cargo.toml` and run `cargo fetch`). Then read the fetched crate's `src/error.rs` and `src/updater.rs` under `~/.cargo/registry/src/` to confirm the `Error` variant names, `Update::download(on_chunk: FnMut(usize, Option<u64>), on_finish: FnOnce())`, `Update::install(bytes)` and `Update.body`; if a name differs from the spec, use the real one and say so in the report.

- [ ] **Step 2: Write the failing tests** in `updates.rs` `mod tests` (build `UpdateState` with struct update syntax over `UpdateState::default()`; `can_update: true` where a check must run; for errors use a constructible variant such as `Error::Io(std::io::Error::other("x"))`):
  - `bundle_type_decides_can_update`: `Deb` → `can_update && deb`; `AppImage` → `can_update && !deb`; `None` → `!can_update`. Also `assert_eq!(tauri::utils::platform::bundle_type(), None)` (tests never update).
  - `wait_for_follows_the_rules` (now = 100_000): setup not done → 60 s; status `Ready` → 60 s; never tried → 0; `last_try = Some((now - 3 * 3600, true))` → 21 h; `Some((now - 600, false))` → 50 min.
  - `error_mapping_matches_the_table`: download + a signature variant (e.g. `SignatureUtf8(String::from_utf8(vec![0xff]).unwrap_err())`) → `BadDownload`; download + `Io` → `DownloadFailed`; install + `InvalidUpdaterFormat` → `BadDownload`; install + `PackageInstallFailed` (or `Io`) → `NotInstalled`.
  - `failed_auto_check_keeps_status_and_shows_nothing`: `UpToDate`, `checked_at: Some(50)`; `begin_check()` → `Some(UpToDate)` and status `Checking`; `finish_check(Err(io), UpToDate, false, 200)` → status `UpToDate`, `error` `None`, `checked_at` `Some(50)`, `last_try` `Some((200, false))`.
  - `failed_manual_check_says_offline`: from `Idle` → status `Idle`, `error` `Some(Offline)`.
  - `check_without_update_is_up_to_date`: `finish_check(Ok(None), Idle, true, 200)` → `UpToDate`, `checked_at` `Some(200)`, `last_try` `Some((200, true))`.
  - `no_check_while_busy_or_not_updatable`: second `begin_check()` → `None`; `can_update: false` → `None`.
  - `install_guard`: `Ready` + `start_install(1)` → `Err("Wait for the repo action to finish.")`, status still `Ready`; `Ready` + `start_install(0)` → `Ok`, `Downloading`, `error` `None`; then `start_install(0)` again → `Err` (Review Focus 2); `action_allowed()` is `Err` with the spec's text while `Downloading` and `Installing`, `Ok` for `Ready` with `error: Some(NotInstalled)` (after a failed install).
  - `mb_step_publishes_on_whole_mb`: `mb_changed(0, 500)` true, `(500, 900_000)` false, `(900_000, 1_000_100)` true, `(1_000_100, 1_999_999)` false, `(1_999_999, 2_000_000)` true.

- [ ] **Step 3: Run them to verify they fail**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p repobell --lib updates`
Expected: does not compile (`updates` items missing).

- [ ] **Step 4: Implement** the interfaces above.

- [ ] **Step 5: Run all checks** (Rust checks from Global Constraints). Expected: all pass, no warnings.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/Cargo.toml Cargo.lock app/src-tauri/tauri.conf.json app/src-tauri/src/lib.rs app/src-tauri/src/watcher.rs app/src-tauri/src/updates.rs
git commit -m "feat(updates): add the updater plugin and update state"
```

### Task 2: Check, timer and link commands — complex

Label reason: async logic and a timer loop.

**Files:**
- Modify: `app/src-tauri/src/updates.rs`, `app/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `begin_check`, `finish_check`, `wait_for` (Task 1); `watcher::publish`, `watcher::now_seconds`, `SharedEngine`.
- Produces in `updates.rs`:
  - `pub async fn check(app: &AppHandle, engine: &SharedEngine, manual: bool)`: under the lock `begin_check()`; `None` → return; else publish, drop the lock, `let found = match app.updater() { Ok(u) => u.check().await, Err(e) => Err(e) };` (`use tauri_plugin_updater::UpdaterExt`), `eprintln!("repobell: update check failed: {e}")` on `Err`, then under the lock `finish_check(found, before, manual, now_seconds())` and publish.
  - `pub async fn scheduler(app: AppHandle, engine: SharedEngine)`: reads `can_update` once and returns when false. Loop: under the lock `wait_for(st.settings.setup_done, &st.update, now_seconds())`; zero → `check(&app, &engine, false).await`; else `tokio::time::sleep(wait.min(60 s))`.
  - `#[tauri::command] pub async fn check_for_updates(app: AppHandle, engine: State<'_, SharedEngine>) -> Result<(), String>`: spawns `check(.., true)` with clones and returns `Ok(())` at once.
  - `#[tauri::command] pub fn open_link(target: String) -> Result<(), String>`: `"source"` / `"issues"` → `std::process::Command::new("xdg-open").arg(url).spawn()` (map the error to a plain sentence); anything else → `Err`.
- `lib.rs`: add both commands to `generate_handler!`; spawn `updates::scheduler(handle.clone(), engine.clone())` next to `watcher::scheduler`.

- [ ] **Step 1: Write the failing test** in `updates.rs` `mod tests`: `open_link_refuses_unknown_targets`: `open_link("https://evil.example".into())` is `Err`. (Don't call it with `source` in tests: it would open a browser.)

- [ ] **Step 2: Run it to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p repobell --lib updates::tests::open_link_refuses_unknown_targets`
Expected: does not compile (`open_link` missing).

- [ ] **Step 3: Implement** the interfaces above. The async wiring has no Tauri test runtime in this crate; the pure rules are tested in Task 1 and the end-to-end test covers the rest.

- [ ] **Step 4: Run all checks** (Rust checks). Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/updates.rs app/src-tauri/src/lib.rs
git commit -m "feat(updates): check for updates at start and every 24 hours"
```

### Task 3: Install task and the repo action guard — complex

Label reason: data loss risk (restart mid-action), async, security (signature errors).

**Files:**
- Modify: `app/src-tauri/src/updates.rs`, `app/src-tauri/src/actions.rs`, `app/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `start_install`, `action_allowed`, `error_for`, `mb_changed` (Task 1); `EngineState.actions_running`.
- Produces:
  - `actions.rs`: `pub struct ActionGuard { app: AppHandle, engine: SharedEngine }` and `pub async fn begin_action(app: &AppHandle, engine: &SharedEngine) -> Result<ActionGuard, String>`: under the lock `st.update.action_allowed()?`, `actions_running += 1`, publish. `impl Drop for ActionGuard`: `tauri::async_runtime::spawn` a task that takes the lock, `actions_running = actions_running.saturating_sub(1)`, publishes.
  - First line of `update_repo`, `undo_repo`, `delete_old_branches`, `undo_cleanup` and `update_main`: `let _action = begin_action(&app, engine.inner()).await?;` Never `let _ =` (Review Focus 1). No other command takes it.
  - `updates.rs`: `#[tauri::command] pub async fn install_update(app: AppHandle, engine: State<'_, SharedEngine>) -> Result<(), String>`: under the lock `st.update.start_install(st.actions_running)?`, clone `pending` (`Ready` always has it; if it is somehow `None`, set `Idle` and return `Err("There's no update to install.")`), publish, spawn `install(app, engine, update)`, return `Ok(())`.
  - `async fn install(app: AppHandle, engine: SharedEngine, update: Update)`, per the spec's install task:
    1. `update.download(on_chunk, || {})`. `on_chunk` keeps a local byte count; when `mb_changed(before, after)` it spawns a task that takes the lock, sets `downloaded = downloaded.max(after)` and `total`, and publishes. (`blocking_lock` would panic inside the async runtime; `max` keeps the bar from going back if two tasks run out of order.)
    2. `Err(e)` → under the lock `status = Ready`, `error = Some(error_for(&e, false))`, publish, return.
    3. `Installing`, publish. `spawn_blocking(move || update.install(bytes))`.
    4. `Ok(Ok(()))` → `app.restart()`. Otherwise `Ready`, `error = Some(error_for(&e, true))` (a `JoinError` → `NotInstalled`), publish.
  - `lib.rs`: add `updates::install_update` to `generate_handler!`.

- [ ] **Step 1: No new unit test.** The rules (`start_install`, `action_allowed`, `error_for`, `mb_changed`) are tested in Task 1; the wiring needs a Tauri runtime this crate has no tests for, and the end-to-end test covers it.

- [ ] **Step 2: Run the existing tests first**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test --workspace`
Expected: pass (baseline).

- [ ] **Step 3: Implement** the interfaces above.

- [ ] **Step 4: Verify the guard is bound in all five commands**

Run: `grep -n "begin_action" app/src-tauri/src/actions.rs`
Expected: the definition plus exactly five lines, each `let _action = begin_action(`.

- [ ] **Step 5: Run all checks** (Rust checks). Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/updates.rs app/src-tauri/src/actions.rs app/src-tauri/src/lib.rs
git commit -m "feat(updates): install updates and never during a repo action"
```

### Task 4: Tray line "Install RepoBell 0.2.0…" — complex

Label reason: adds a branch to the tray rebuild logic.

**Files:**
- Modify: `app/src-tauri/src/news.rs`, `app/src-tauri/src/watcher.rs`, `app/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `EngineState.update.pending` (Task 1).
- Produces:
  - `news.rs`: `TrayModel` gains `pub install: Option<String>`; `tray_model` sets it to `None`. `pub fn install_label(version: &str) -> String` → `format!("Install RepoBell {version}…")`.
  - `watcher.rs` `publish`: after building `model`, `model.install = st.update.pending.as_ref().map(|u| news::install_label(&u.version));` before the comparison. `apply_tray`: when `model.install` is `Some(text)`, add `MenuItem::with_id(app, "install_update", text, true, None::<&str>)` between the last separator and Quit.
  - `lib.rs` `on_menu_event`: `"install_update"` → `show_and_position_window(&w)` then `w.emit("open-about", ())`.

- [ ] **Step 1: Write the failing tests** in `news.rs` `mod tests`:
  - `install_label_names_the_version`: `install_label("0.2.0") == "Install RepoBell 0.2.0…"`.
  - In `tray_follows_the_same_rule` (or a new test): `tray_model(…).install` is `None`. Existing `TrayModel` literals in tests gain `install: None`.

- [ ] **Step 2: Run them to verify they fail**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo test -p repobell --lib news`
Expected: does not compile.

- [ ] **Step 3: Implement** the interfaces above.

- [ ] **Step 4: Run all checks** (Rust checks). Expected: all pass, existing tray tests included.

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/news.rs app/src-tauri/src/watcher.rs app/src-tauri/src/lib.rs
git commit -m "feat(tray): add the install line when an update is ready"
```

### Task 5: Window types, test default and `checkedAt` — complex

Label reason: touches 4 files and adds date branches.

**Files:**
- Modify: `app/src/lib/types.ts`, `app/src/test-helpers.ts`, `app/src/lib/util.ts`
- Create: `app/src/lib/__tests__/util.test.ts`

**Interfaces:**
- Produces:
  - `types.ts`: `export interface UpdateView` exactly as in the spec, and `update: UpdateView` on `AppStateView`.
  - `test-helpers.ts`: `export function makeUpdate(overrides: Partial<UpdateView> = {}): UpdateView` with defaults `{ current: "0.1.0", can_update: true, deb: true, status: "idle", checked_at: null, available: null, downloaded: 0, total: null, error: null, busy: false }`; `makeState` default `update: makeUpdate()`.
  - `util.ts`: `export function checkedAt(timestamp: number, now: number = Date.now()): string` (timestamp in Unix seconds, `now` in ms, like `timeAgo`). Local time. Same calendar day → `today at HH:MM`; the day before → `yesterday at HH:MM`; else `on D Mon at HH:MM` with a fixed English month array (`"Jan"`…`"Dec"`, no locale API: ICU prints "Sept"). 24-hour, zero-padded.

- [ ] **Step 1: Write the failing tests** in `util.test.ts` (build times with `new Date(y, m, d, h, min)`; `const s = (d: Date) => d.getTime() / 1000`):
  - `today`: ts 2026-11-17 09:12, now 2026-11-17 15:00 → `"today at 09:12"`.
  - `yesterday`: ts 2026-11-16 23:59 → `"yesterday at 23:59"`; and across a month: ts 2026-11-30 22:05, now 2026-12-01 08:00 → `"yesterday at 22:05"`.
  - `older`: ts 2026-11-14 09:12, now 2026-11-17 15:00 → `"on 14 Nov at 09:12"`.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/lib/__tests__/util.test.ts`
Expected: FAIL (`checkedAt` is not exported).

- [ ] **Step 3: Implement** the interfaces above.

- [ ] **Step 4: Run all checks**

Run: `cd app && npm run check && npm test`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/types.ts app/src/test-helpers.ts app/src/lib/util.ts app/src/lib/__tests__/util.test.ts
git commit -m "feat(ui): add the update state type and the checked-at text"
```

### Task 6: About section in Settings — complex

Label reason: many state branches, 4 files.

Load `impeccable` and read `/home/ramin/projects/repo-bell/.claude/skills/impeccable/reference/craft-floor.md` before the first UI edit. Follow DESIGN.md tokens.

**Files:**
- Create: `app/src/lib/AboutSection.svelte`, `app/src/lib/__tests__/AboutSection.test.ts`
- Modify: `app/src/lib/SettingsView.svelte`, `app/src/lib/__tests__/SettingsView.test.ts` (only if its renders need the new props)

**Interfaces:**
- Consumes: `UpdateView`, `makeUpdate`, `checkedAt` (Task 5); commands `check_for_updates`, `install_update`, `open_link` (Tasks 2, 3).
- Produces:
  - `AboutSection.svelte` props: `export let update: UpdateView; export let now: number; export let aboutRequest = 0;`
  - Markup: `<section class="section" aria-labelledby="about-title">` with `<h2 class="section-title" id="about-title" tabindex="-1">About</h2>`, then the card per the spec (Surface, Hairline border, 10px radius, 12px padding): bell icon (reuse the 24×24 bell path from `WelcomeView.svelte`), "RepoBell {update.current}" (Ink 700), one status line (13px), one button on the right; the notes box; the links row.
  - Status line and button: exactly the spec's state table. `version` = `update.available?.version`. Download line: done = `Math.floor(downloaded / 1e6)`, total = `Math.ceil(total / 1e6)`; "Downloading {v}… {done} of {total} MB", or "Downloading {v}… {done} MB" with no total. Installing line by `update.deb`. `can_update: false` → the built-from-source line, no button, ignores status. When `error` is set its line and a Try again button replace the state's.
  - Error lines: `role="alert"` on all four; Alert Red (#a12a20) for `offline`, `download_failed`, `bad_download`; `not_installed` stays the normal line color (as in the mockup).
  - Buttons (mockup styles): Install and the `not_installed` Try again are Primary; Check for updates and the other Try again are the quiet button. Check for updates is disabled while `checking`. Install is disabled while `busy`, with `<p id="install-wait">Wait for the repo action to finish.</p>` (12px Muted Ink) under it and `aria-describedby="install-wait"`.
  - Clicks: Check → `invoke("check_for_updates")`; Install → `invoke("install_update")`; Try again → `check_for_updates` when `error === "offline"`, else `install_update`. Results are ignored (`.catch(() => {})`); the state arrives by `state-updated`.
  - Download bar under the line while `downloading`: the status strip's bar (4px, Chip Grey track, Muted Ink fill, 2px radius, width eases 0.15s, none under `prefers-reduced-motion: reduce`); `role="progressbar"`, `aria-label="Download progress"`, `aria-valuemin="0"`, `aria-valuemax={totalMb}` only with a total, `aria-valuenow={doneMb}`, `aria-valuetext` = "{done} of {total} MB" (or "{done} MB"). Width `downloaded / total`, 0 with no total.
  - Notes box ("What's new in {version}", Canvas fill, 8px radius, 13px Soft Ink): only while `available` and the parsed list is not empty. Parse: split on newlines, trim, drop a leading `- ` or `* `, drop empty lines, render `<li>{line}</li>`. Never `{@html}`.
  - Links row under a Hairline: two `<button>`s styled as Signal Blue underlined links, "Source code" → `invoke("open_link", { target: "source" })`, "Report a problem" → `{ target: "issues" }`; "MIT license" plain Muted Ink text.
  - Jump: when `aboutRequest > 0` (on mount and on each change), `await tick()`, `section.scrollIntoView?.({ block: "start" })`, focus the heading.
  - `SettingsView.svelte`: new props `export let now: number = Date.now(); export let aboutRequest = 0;`; render `<AboutSection update={state.update} {now} {aboutRequest} />` as the last child of `.settings-body`.

- [ ] **Step 1: Write the failing tests** in `AboutSection.test.ts` (`render(AboutSection, { update: makeUpdate({...}), now })`; `mockIPC` records calls; `const ready = { version: "0.2.0", notes: "- Shows the version\n\n* Faster checks" }`):
  - `it.each` over the state rows: idle → no line, button "Check for updates"; checking → "Checking for updates…", button disabled; up_to_date with `checked_at` today 09:12 → "Up to date. Checked today at 09:12"; ready → "Version 0.2.0 is ready", button "Install"; downloading `downloaded: 4_200_000, total: 8_100_000` → "Downloading 0.2.0… 4 of 9 MB", no button; installing `deb: true` → the password line; installing `deb: false` → "Installing 0.2.0… RepoBell restarts after."; each error (`offline`, `download_failed`, `bad_download`, `not_installed`) → its line in a `role="alert"` and "Try again".
  - `always shows the version`: "RepoBell 0.1.0" in every row above.
  - `notes are split into items and only shown when not empty`: `ready` → `getAllByRole("listitem")` texts `["Shows the version", "Faster checks"]` under "What's new in 0.2.0"; `notes: "  \n"` → no "What's new".
  - `notes are plain text` (Review Focus 3): notes `"<b>bold</b>"` → a list item with text `"<b>bold</b>"` and no `b` element in the section.
  - `install waits for a repo action`: ready + `busy: true` → Install disabled, `aria-describedby` points to an element with "Wait for the repo action to finish."
  - `try again repeats the step that failed`: `offline` → click calls `check_for_updates`; `bad_download` (with `available`, status `ready`) → calls `install_update`.
  - `download bar without a total`: `total: null, downloaded: 4_200_000` → "Downloading 0.2.0… 4 MB"; progressbar has no `aria-valuemax`, `aria-valuetext "4 MB"`.
  - `links open through the backend`: clicks call `open_link` with `{ target: "source" }` and `{ target: "issues" }`.
  - `built from source`: `can_update: false` → "This copy was built from source, so it doesn't update itself.", no button.
  - `jumps to About when asked`: stub `Element.prototype.scrollIntoView = vi.fn()`; render with `aboutRequest: 1` → the stub was called and `document.activeElement` is the "About" heading.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/lib/__tests__/AboutSection.test.ts`
Expected: FAIL (component missing).

- [ ] **Step 3: Implement** the interfaces above.

- [ ] **Step 4: Run all checks**

Run: `cd app && npm run check && npm test`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add app/src/lib/AboutSection.svelte app/src/lib/__tests__/AboutSection.test.ts app/src/lib/SettingsView.svelte
git commit -m "feat(ui): add the About section at the end of Settings"
```
(Add `app/src/lib/__tests__/SettingsView.test.ts` only if it changed.)

### Task 7: Footer button and the jump to About — complex

Label reason: touches 4 files and adds view state.

Load `impeccable` and read `/home/ramin/projects/repo-bell/.claude/skills/impeccable/reference/craft-floor.md` before the first UI edit.

**Files:**
- Modify: `app/src/lib/Footer.svelte`, `app/src/App.svelte`
- Test: `app/src/lib/__tests__/Footer.test.ts`, `app/src/__tests__/App.test.ts`

**Interfaces:**
- Consumes: `state.update` (Task 5), `SettingsView` props `now` and `aboutRequest` (Task 6), the `open-about` event (Task 4).
- Produces:
  - `Footer.svelte`: `export let update: UpdateView | null = null; export let onOpenAbout: () => void = () => {};`. When `update?.can_update && update.available`: `<button class="foot-update" onclick={onOpenAbout}><span class="pip" aria-hidden="true"></span>RepoBell {update.available.version} is ready</button>` on the right: 14px, weight 500, Signal Blue text, 8px Muted Ink round dot before it.
  - `App.svelte`: `let aboutRequest = 0;` `function openAbout() { currentView = "settings"; aboutRequest += 1; }` `function openSettings() { aboutRequest = 0; currentView = "settings"; }` used by the gear and the "Open Settings" button. `listen("open-about", openAbout)` next to `focus-repo` (unlisten on destroy). `<SettingsView {state} {now} {aboutRequest} …>` and `<Footer onAddFolder={handleAddFolder} update={state.update} onOpenAbout={openAbout} />`.

- [ ] **Step 1: Write the failing tests**
  - `Footer.test.ts`: keep the existing test (no `update` → one button); `shows the ready button and calls onOpenAbout`: `update: makeUpdate({ status: "ready", available: { version: "0.2.0", notes: "" } })` → button "RepoBell 0.2.0 is ready"; click → `onOpenAbout` called once; `no button when the copy can't update`: same with `can_update: false` → one button.
  - `App.test.ts`, new `describe("about")` (state with a ready update; `Element.prototype.scrollIntoView = vi.fn()`; mock `detect_editors`):
    - `the footer button opens Settings with About focused`: click "RepoBell 0.2.0 is ready" → `findByText("Folders and repos")`, then `document.activeElement` is the "About" heading.
    - `open-about opens About, also when Settings is open` (`shouldMockEvents: true`): `emit("open-about")` from the main view → About focused; move focus to the Back button; emit again while in Settings → About focused again.
    - `the gear opens Settings at the top after a jump` (Review Focus 4): open About from the footer, go Back, click the gear → focus is not the About heading and `scrollIntoView` was not called after the gear click.
    - `no footer button during the welcome`: `first_run: true` state with a ready update → no "is ready" button.

- [ ] **Step 2: Run them to verify they fail**

Run: `cd app && npx vitest run src/lib/__tests__/Footer.test.ts src/__tests__/App.test.ts`
Expected: the new tests fail.

- [ ] **Step 3: Implement** the interfaces above.

- [ ] **Step 4: Run all checks**

Run: `cd app && npm run check && npm test`
Expected: all pass.

- [ ] **Step 5: Screenshots.** Per the spec, take 420×640 screenshots of About (up to date, ready with notes, downloading) and the footer line. Use `npm run dev` in the worktree with a browser and a mocked IPC page, or a test build; never start a RepoBell that uses the user's data dirs (if you must run the app, set `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_RUNTIME_DIR` to `mktemp -d` folders). Save them under `artifacts/` (git-ignored) and list the paths in the report.

- [ ] **Step 6: Commit**

```bash
git add app/src/lib/Footer.svelte app/src/App.svelte app/src/lib/__tests__/Footer.test.ts app/src/__tests__/App.test.ts
git commit -m "feat(ui): show a ready update in the footer and jump to About"
```

### Task 8: Signed release, `latest.json` and the release script — complex

Label reason: security (signing, the manifest installed copies trust).

**Files:**
- Create: `scripts/release-files.sh` (executable)
- Modify: `.github/workflows/release.yml`

**Interfaces:**
- `scripts/release-files.sh <tag> <notes file> <download base URL> <out dir>`, `#!/usr/bin/env bash`, `set -euo pipefail`, run from the repo root. `v="${tag#v}"`. Reads `target/release/bundle/deb/RepoBell_${v}_amd64.deb{,.sig}` and `target/release/bundle/appimage/RepoBell_${v}_amd64.AppImage{,.sig}`. Writes into `<out dir>` (created): both versioned files, fixed-name copies `RepoBell_amd64.deb` and `RepoBell_amd64.AppImage`, and `latest.json` via `jq -n` with `--arg`/`--rawfile`, shape exactly as in the spec. `notes` and both `signature` values have trailing whitespace removed (`sub("\\s+$"; "")`). `pub_date` from `date -u +%Y-%m-%dT%H:%M:%SZ`. Keys only `linux-x86_64-deb` and `linux-x86_64-appimage`. No `.sig` files in the out dir. A missing input file fails the script (non-zero).
- `release.yml`, per the spec's "Release workflow" section: Build step gets `shell: bash`, the two `env` lines, the `--config` build with `tee build.log` and the `grep -E` guard. New "Release notes" step (repo root) with the spec's tag fetch and `notes.txt`. New "Release files" step running the script with `"https://github.com/raminmz66/repo-bell/releases/download/$GITHUB_REF_NAME"` and `release-files`. softprops: `files: release-files/*`, `body_path: notes.txt`, keep `generate_release_notes: true`. The tag check step stays as it is.

- [ ] **Step 1: Write the local check first** (not committed): a shell snippet that makes a fake repo root with `mktemp -d`, creates `target/release/bundle/deb/RepoBell_0.1.1_amd64.deb` and `appimage/RepoBell_0.1.1_amd64.AppImage` (any bytes), their `.sig` files with `printf 'c2lnLWRlYg==\n'` and `printf 'c2lnLWFwcA=='`, and `notes.txt` with `printf 'Shows the version\nFaster checks\n'`; then `(cd "$fake" && "$repo/scripts/release-files.sh" v0.1.1 notes.txt http://127.0.0.1:8765 out)`.

- [ ] **Step 2: Run it to verify it fails**

Expected: "No such file or directory" (script missing).

- [ ] **Step 3: Implement** the script and the workflow changes.

- [ ] **Step 4: Verify the script against the fake folder**

Run the snippet, then:
```bash
ls "$fake/out"   # RepoBell_0.1.1_amd64.deb RepoBell_0.1.1_amd64.AppImage RepoBell_amd64.deb RepoBell_amd64.AppImage latest.json (and nothing else)
jq -e '.version == "0.1.1"
  and .notes == "Shows the version\nFaster checks"
  and (.pub_date | test("^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$"))
  and (.platforms | keys == ["linux-x86_64-appimage", "linux-x86_64-deb"])
  and .platforms["linux-x86_64-deb"].url == "http://127.0.0.1:8765/RepoBell_0.1.1_amd64.deb"
  and .platforms["linux-x86_64-deb"].signature == "c2lnLWRlYg=="
  and .platforms["linux-x86_64-appimage"].signature == "c2lnLWFwcA=="' "$fake/out/latest.json"
cmp "$fake/out/RepoBell_amd64.deb" "$fake/target/release/bundle/deb/RepoBell_0.1.1_amd64.deb"
```
Expected: the listed files, `true`, `cmp` silent. Then delete the `.AppImage.sig` and rerun: the script exits non-zero. Then an empty notes file: `.notes == ""`.

- [ ] **Step 5: Check the workflow file**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml'))" && bash -n scripts/release-files.sh && shellcheck scripts/release-files.sh` (skip `shellcheck` if not installed; say so).
Expected: no output, exit 0.

- [ ] **Step 6: Commit**

```bash
chmod +x scripts/release-files.sh
git add scripts/release-files.sh .github/workflows/release.yml
git commit -m "ci: sign releases and publish latest.json for updates"
```

### Task 9: Docs — complex

Label reason: touches 5 files (no logic).

**Files:**
- Modify: `docs/decisions.md`, `README.md`, `CONTRIBUTING.md`, `DESIGN.md`, `ROADMAP.md`

- [ ] **Step 1: `docs/decisions.md`**: add the spec's "About and updates" section (from "Decision text for docs/decisions.md") as written, after "Check progress" and before "## Design reviews". Replace the body of "## Releases" with the spec's new text.

- [ ] **Step 2: `README.md`** "Install": replace the manual download text with the spec's one-line `.deb` command (`wget … && sudo apt install -y /tmp/RepoBell.deb`), the AppImage lines from the spec, and one line on updates ("RepoBell checks for a new version once a day; Settings, About installs it"). Keep the remove line.

- [ ] **Step 3: `CONTRIBUTING.md` 4.3**: tag with `git tag -a vX.Y.Z -m "…"` for notes (a plain tag gives none); local builds don't sign and don't need the key; the release needs the `TAURI_SIGNING_PRIVATE_KEY` secret; don't add a .deb dependency without telling users to reinstall.

- [ ] **Step 4: `DESIGN.md`**: under Components, the footer update button (14px, 500, Signal Blue text, 8px Muted Ink dot; blue text, grey dot) and the About card (Surface, Hairline, 10px radius, 12px padding; notes box Canvas, 8px radius; links Signal Blue underlined; download bar = the status strip bar). Add `about-card` and `footer-update` to the front matter `components`.

- [ ] **Step 5: `ROADMAP.md`**: add under Fixes "About and in-app updates" with Design, Plan and Build ticked and open items "End-to-end updater test", "Release v0.1.0", "Make the repo public", "Release v0.1.1 and update from the app". Set "Current step" to the build being done and waiting for the end-to-end test.

- [ ] **Step 6: Commit**

```bash
git add docs/decisions.md README.md CONTRIBUTING.md DESIGN.md ROADMAP.md
git commit -m "docs: record About and in-app updates"
```
