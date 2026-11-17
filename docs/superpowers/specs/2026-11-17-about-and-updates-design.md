# About and in-app updates design

Date: 2026-11-17. Status: written for Ramin's review. Ramin decided on 2026-11-17: the repo goes public, downloads come from GitHub Releases, updates install from inside the app for .deb and AppImage, updates are signed, the UI is mockup option B with the review fixes listed below, and the release order (build this, release v0.1.0 with it, then make the repo public). Everything marked "(Claude's call)" is open to change.

Visual target: option B "Quiet footer button, About at the end of Settings" and the "Update states" row in `artifacts/about-and-updates.html` (this worktree, git-ignored). Where this spec and the mockup differ, the spec wins.

## Why

Nothing is released yet. A teammate who wants RepoBell today has to build it from source. Once they have it, nothing tells them a new version exists, and nothing shows which version they run.

## Goals

- A teammate installs RepoBell with one command, without a GitHub account.
- RepoBell finds a new version by itself and installs it after one click and, for the .deb, the Ubuntu password.
- Settings shows the version, the update status and links to the source and to issues.
- A changed or broken download is never installed.

## Non-goals

- Installing without a click.
- A "check automatically" switch (left out on purpose).
- A desktop notification for a new version.
- rpm, Snap, Flatpak or an apt repository.
- Beta channels, downgrades or rollback.

## Anti-goals

The change fails, even if it works, if any of these happen:

- An update is installed without a valid signature from our key.
- RepoBell restarts while Update, Undo, a branch delete or Update main is running, so a stash is left behind or a result is lost.
- The password prompt comes as a surprise (the tray line installs straight away, or a timer installs).
- An automatic check that fails shows an error.
- Blue is used for the update dot (blue is team news).

## Approaches

Where the update logic runs (Claude's call):

1. Rust side (picked). One module owns the state, the 24-hour timer, the download and the restart. The tray line needs this state anyway, and the existing `publish` already sends state to the window and the tray. The webview gets no updater permissions, so web code can't start an install by itself.
2. JS side with `@tauri-apps/plugin-updater` and `@tauri-apps/plugin-process`. Two npm packages, capability entries, and the tray and the timer still need Rust code.

How `latest.json` is made (Claude's call):

1. Keep `softprops/action-gh-release` and add a small script that writes `latest.json` with `jq` (picked). About 20 lines. The same script runs in the local end-to-end test, so the manifest is tested before a real release.
2. Switch to `tauri-apps/tauri-action`. It builds, uploads and writes `latest.json`, but it replaces the build step we already trust, and the fixed-name copies for the install command still need an extra step.

Where the release notes come from (Claude's call):

1. The message of an annotated tag (picked): `git tag -a v0.2.0 -m "Shows the version and installs updates" -m "Faster checks for many repos"`. No new file. A plain tag (`git tag v0.2.0`) gives no notes, and the About section then shows none.
2. A `CHANGELOG.md` section per version. Easy to forget to update, and old notes would show for a new version.
3. GitHub's generated notes. They are pull request titles with links and names, not plain words for designers.

## What the user sees

### About section (end of Settings)

A new last section in Settings, "About", in a white card (Surface, Hairline border, 10px radius, 12px padding), as in the mockup:

- Top row: the bell icon the window already uses, then "RepoBell 0.1.0" (Ink, 700) with one status line under it (13px), then one button on the right.
- "What's new in 0.2.0" box under the top row, only while an update is known and its notes are not empty. Canvas fill, 8px radius, 13px Soft Ink, notes as a bullet list.
- Links row under a Hairline: "Source code" (`https://github.com/raminmz66/repo-bell`), "Report a problem" (`https://github.com/raminmz66/repo-bell/issues/new`), and "MIT license" as plain Muted Ink text. Links are Signal Blue and underlined. They open in the default browser.

Status line and button per state. The copy comes from the mockup, except where marked new:

| State | Status line | Button |
|---|---|---|
| Not checked yet (idle) | none | Check for updates |
| Checking | Checking for updates… | Check for updates, disabled |
| Up to date | Up to date. Checked today at 09:12 | Check for updates |
| Ready | Version 0.2.0 is ready (Signal Blue) | Install |
| Ready, a repo action runs | Version 0.2.0 is ready | Install, disabled, with "Wait for the repo action to finish." under it (new, 12px Muted Ink) |
| Downloading | Downloading 0.2.0… 4 of 9 MB, with a 4px bar under it | none |
| Installing, .deb | Enter your password in the Ubuntu window to finish. RepoBell restarts after. | none |
| Installing, AppImage | Installing 0.2.0… RepoBell restarts after. (new) | none |
| Password window closed or install failed | Update not installed. Nothing was changed. | Try again |
| Check failed (offline or GitHub down) | Can't reach GitHub to check for updates | Try again |
| Download stopped | The download stopped. Check your connection and try again. (new) | Try again |
| Signature or file check failed | The download didn't pass the safety check, so it wasn't installed | Try again |
| Can't update itself (dev build, built from source) | This copy was built from source, so it doesn't update itself. (new) | none |

- "Checked today at 09:12": "today" or "yesterday", else the date ("on 14 Nov"), and the time as 24-hour `HH:MM`. (Claude's call.)
- The download bar is the status strip's bar: Chip Grey track, Muted Ink fill. Never blue. MB are whole numbers: done rounded down, total rounded up. With no size from the server, the line reads "Downloading 0.2.0… 4 MB".
- When `error` is set, its line and Try again replace the state's own line and button.
- Try again repeats the step that failed: the check after "Can't reach GitHub", the install otherwise.
- The three error lines are Alert Red (#a12a20).
- After a good install RepoBell restarts. The window opens again by itself, as it does at every start.

### Footer (main window)

- When an update is known, the empty right side of the footer shows a text button "RepoBell 0.2.0 is ready": 14px, weight 500, Signal Blue text, with an 8px Muted Ink dot before it. The dot is grey on purpose, because blue is for team news.
- It shows for as long as an update is known: while ready, downloading, installing and after a failed install. Otherwise the footer shows only Add folder, as today.
- Clicking it opens Settings scrolled to About, with focus on the About heading.

### Tray menu

- When an update is known, an item "Install RepoBell 0.2.0…" sits between the last separator and Quit.
- It opens the window on Settings, scrolled to About. It never installs by itself, so the password prompt never comes as a surprise.

### When RepoBell checks

- At start and then every 24 hours. A failed automatic check tries again after 1 hour, because at login the network is often not up yet. (Claude's call.)
- Not while the first-run welcome is open. The first check runs once setup is done.
- An automatic check that fails changes nothing on screen. Errors show only after the user pressed "Check for updates" or Try again.
- No check while an update is already known. After the restart the new version checks again.
- A build that can't update itself never checks.

## How it works

### Packages and config

Checked on 2026-11-17 against crates.io, npm and the published crate source (`tauri-plugin-updater-2.14.1.crate`, the same `updater.rs` as branch `v2`):

- Add `tauri-plugin-updater = "2.14"` to `app/src-tauri/Cargo.toml`, default features. 2.14.1 needs `tauri` 2.12 and Rust 1.90. We lock `tauri` 2.12.5 and build with Rust 1.99, so nothing else moves.
- No `tauri-plugin-process`: `AppHandle::restart()` is in `tauri` itself. No npm packages. No capability entries, because the webview never calls the plugin's commands (its commands exist, but `capabilities/default.json` does not grant `updater:default`).
- `tauri.conf.json` gets:

  ```json
  "plugins": {
    "updater": {
      "pubkey": "<contents of repobell.key.pub>",
      "endpoints": ["https://github.com/raminmz66/repo-bell/releases/latest/download/latest.json"],
      "requireSignedVersion": true
    }
  }
  ```

  `pubkey` is required: the plugin fails to start without it. `requireSignedVersion` makes the plugin reject a signature that doesn't name the announced version, so an edited `latest.json` can't pair "0.3.0" with an older signed file. Safe to turn on: our CLI (2.12.4) signs with the app version (`sign_file(..., Some(settings.version_string()))` in `tauri-cli/src/bundle.rs`).
- `bundle.createUpdaterArtifacts` stays off in `tauri.conf.json`. The release workflow turns it on with `--config`. With it on, `tauri build` fails when `TAURI_SIGNING_PRIVATE_KEY` is missing, so turning it on in the file would break every local build and Ramin's reinstall flow. (Checked in `tauri-cli/src/interface/rust.rs` and `bundle.rs`.)
- Register the plugin in `lib.rs`: `.plugin(tauri_plugin_updater::Builder::new().build())`.

### Which package am I?

- `tauri::utils::platform::bundle_type()` returns `Deb`, `AppImage` or `None`. The bundler writes the type into a copy of the binary for each package it builds (`patch_binary` in `tauri-bundler/src/bundle.rs`, our CLI 2.12.4 has it; `tauri-utils` 2.10.2 in our lock reads it). `tauri dev`, `cargo run` and a plain `cargo build` give `None`.
- `can_update = bundle_type() is Deb or AppImage`. `deb = bundle_type() is Deb`. Both are read once at start.
- The plugin uses the same value. It looks up `linux-x86_64-deb` or `linux-x86_64-appimage` in `latest.json`, then `linux-x86_64`. We publish only the first two, so a .deb never gets the AppImage.
- A locally built .deb (Ramin's reinstall flow) has the type too, so it updates from GitHub like a downloaded one.

### Update state (Rust)

New module `app/src-tauri/src/updates.rs`. The state lives in `EngineState` as `update: UpdateState`, so it goes out with the existing `publish` (window event and tray) and with `get_app_state`. No new event for the state.

`UpdateState` holds:

- `current` (the app version from `app.package_info().version`; `package_info` reads `tauri.conf.json`, the single source), `can_update`, `deb`. `EngineState::new` starts with `UpdateState::default()` (`can_update` false), and `setup` fills these three with `blocking_lock`.
- `status`: `Idle`, `Checking`, `UpToDate`, `Ready`, `Downloading`, `Installing`.
- `checked_at: Option<u64>`: Unix seconds of the last check that reached GitHub. In memory only, so it starts empty after a restart until the start check runs (a few seconds). (Claude's call: nothing needs it to survive a restart.)
- `last_try: Option<(u64, bool)>`: when the last check ran and if it worked. Only the timer reads it.
- `pending: Option<tauri_plugin_updater::Update>`: the update found, kept for Install.
- `downloaded: u64`, `total: Option<u64>`.
- `error: Option<UpdateError>`: `Offline`, `DownloadFailed`, `BadDownload`, `NotInstalled`. Set only by steps the user started. Cleared when the next step starts.

`EngineState` also gets `actions_running: u32` (see below).

`AppStateView` gets `update: UpdateView`, built in `to_view` from `self.update` and `self.actions_running`. In `types.ts`:

```ts
export interface UpdateView {
  current: string;          // "0.1.0"
  can_update: boolean;
  deb: boolean;
  status: "idle" | "checking" | "up_to_date" | "ready" | "downloading" | "installing";
  checked_at: number | null; // Unix seconds
  available: { version: string; notes: string } | null; // set while `pending` is set
  downloaded: number;        // bytes
  total: number | null;
  error: "offline" | "download_failed" | "bad_download" | "not_installed" | null;
  busy: boolean;             // actions_running > 0
}
```

A failed install keeps `pending` and goes back to `Ready` with `error` set, so Try again needs no new check.

### Commands

All in `updates.rs`, added to `generate_handler!`. Both update commands return at once; progress and results arrive with `state-updated`.

- `check_for_updates()`: runs a manual check.
- `install_update()`: under the state lock, needs `status == Ready` and `actions_running == 0`, else returns an error string. Sets `Downloading`, publishes, and spawns the install task.
- `open_link(target: "source" | "issues")`: runs `xdg-open` with the fixed URL for that target. The webview can't pass a URL. (Five lines; `editors::spawn` has editor-specific error text, so it isn't reused.)

The check (`check(app, engine, manual)`):

1. Under the state lock: return at once unless `can_update` and the status is `Idle` or `UpToDate`. Set `Checking`, publish. The timer and the button both go through this, so two checks never overlap.
2. `app.updater()?.check().await`.
3. `Ok(Some(update))`: `pending = Some(update)`, `Ready`, `checked_at = now`.
4. `Ok(None)`: `UpToDate`, `checked_at = now`.
5. `Err(e)`: log it with `eprintln!`, go back to the status before the check. If `manual`, set `error = Offline`. Every check error counts as offline, including a missing `latest.json` or a missing platform entry. (Claude's call: rare, and the user can do nothing else anyway.)
6. Set `last_try`, publish.

The install task:

1. `update.download(on_chunk, on_finish)`. `on_chunk` adds to `downloaded`, sets `total`, and publishes only when the whole MB number changes (about 4 publishes for the .deb, 84 for the AppImage). `download` checks the signature before it returns.
2. Set `Installing`, publish.
3. `update.install(bytes)` inside `spawn_blocking`, because for the .deb it waits for the password window.
4. On success: `app.restart()`. It is called off the main thread, so Tauri asks the app to exit and starts the new binary after. The binary path is cached before `main` runs (`tauri-utils` `starting_binary`), so after `dpkg` replaces `/usr/bin/repobell` the new file starts. For the AppImage, `restart` starts `$APPIMAGE`, the file the plugin just replaced.
5. On error: `Ready` with an error, publish.

Error mapping (one `match` on `tauri_plugin_updater::Error`, unit tested):

| Step | Error variants | `error` |
|---|---|---|
| download | `Minisign`, `Base64`, `SignatureUtf8`, `SignedVersionMismatch`, `MissingSignedVersion` | `BadDownload` |
| download | everything else (`Reqwest`, `Network`, `Io`, …) | `DownloadFailed` |
| install | `InvalidUpdaterFormat`, and the signature variants above (`install` checks again) | `BadDownload` |
| install | everything else (`PackageInstallFailed`, `AuthenticationFailed`, `Io`, …) | `NotInstalled` |

### The .deb password, as the plugin does it

Checked in `updater.rs` (`try_install_with_privileges`): the plugin writes the .deb to a temp folder and runs `pkexec dpkg -i`. If that fails or the window is closed, it asks for the password again with `zenity` (already in our .deb's `depends`), then tries plain `sudo`, which fails without a terminal. So closing the first password window shows a second one. Closing both ends with `PackageInstallFailed`, which shows "Update not installed. Nothing was changed." Accepted: changing it would mean writing our own installer.

When RepoBell runs from a terminal, the last `sudo` asks in that terminal and waits. Only developers do this. Accepted.

### Timer

`updates::scheduler(app, engine)`, spawned in `setup` next to `watcher::scheduler`. It returns at once when `can_update` is false. Otherwise it loops, like the check scheduler: it sleeps at most 60 seconds at a time and compares wall-clock time, so a check that fell due during suspend runs within a minute of waking up.

A pure function decides the wait, so it can be tested:

- setup not done: wait 60 s and look again;
- an update is known, or a check or install is running: wait 60 s;
- never tried: check now;
- last try worked: 24 h after it;
- last try failed: 1 h after it.

### Install and repo actions

The restart must never cut a repo action in half (Update with unsaved changes stashes, pulls and pops). The frontend can't track this: rows unmount when Settings opens, and `RepoCard` clears its busy flag in `onDestroy`. So the backend counts.

- New helper in `actions.rs`: `begin_action(app, engine) -> Result<ActionGuard, String>`. Under the state lock it refuses with "RepoBell is installing an update and will restart in a moment." while `update.status` is `Downloading` or `Installing`. Otherwise it adds 1 to `actions_running` and publishes.
- `ActionGuard`'s `Drop` spawns a short task that takes the state lock, subtracts 1 and publishes. `Drop` covers every early return.
- Called first in `update_repo`, `undo_repo`, `delete_old_branches`, `undo_cleanup` and `update_main`. The others don't count: `preview_update`, `refresh_old_branches` and checks never touch the working folder or branches, and Mark all as seen and settings saves write their files atomically, so a restart in the middle loses nothing.
- `install_update` refuses while `actions_running > 0`. Both checks happen under the same state lock, so they can't race.
- Settings, `seen.json` and `notified.json` are already saved atomically on every change, so the restart needs no extra save. Update's Undo and the branch-delete Undo live in memory and go away with the restart, as they do when RepoBell quits. Accepted.

### Tray

- `TrayModel` (news.rs) gets `install: Option<String>`. `tray_model` leaves it `None`. `publish` sets it to `Install RepoBell {version}…` when `update.pending` is set, before it compares models, so the menu is rebuilt only when the line appears or goes.
- `apply_tray` adds the item with id `install_update` after the last separator, before Quit.
- `lib.rs` `on_menu_event`: `install_update` calls `show_and_position_window` and emits `open-about` to the window.

### Window

- `Footer.svelte`: new props `update: UpdateView | null` and `onOpenAbout`. The button shows when `update?.can_update && update.available`.
- `App.svelte`: `openAbout()` sets `currentView = "settings"` and adds 1 to `aboutRequest`. The footer button and a new `open-about` listener (next to `focus-repo`) call it. The Settings gear sets `aboutRequest = 0`, so opening Settings the normal way starts at the top.
- `SettingsView` gets two props from `App`: `now` and `aboutRequest` (default 0).
- New `AboutSection.svelte` with props `update`, `now`, `aboutRequest`. `SettingsView` renders it last and passes `state.update`. When `aboutRequest` changes, and on mount when it is above 0, it waits a tick, calls `scrollIntoView({ block: "start" })` on the section (no smooth scrolling: WebKitGTK ignored it for the tray jump) and focuses the heading (`tabindex="-1"`).
- Buttons call `invoke("check_for_updates")`, `invoke("install_update")` or `invoke("open_link", { target })`. Try again picks the command from `update.error`. The window ignores what these return; the result arrives in the next `state-updated`.
- Notes: split into lines, trim, drop a leading `- ` or `* `, drop empty lines, show as `<li>`. Plain text only, never `{@html}`.
- New `checkedAt(ts, now)` in `util.ts` for "today at 09:12".

## Release and install

### Release workflow (`.github/workflows/release.yml`)

The tag check stays as it is. Changes:

1. Build step: add `env` with `TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ""`, use `shell: bash` (so `pipefail` is on), and run

   ```bash
   npm run tauri build -- --config '{"bundle":{"createUpdaterArtifacts":true}}' 2>&1 | tee build.log
   if grep -E "does not match the public key|Failed to add bundler type" build.log; then exit 1; fi
   ```

   Both lines are only warnings in the CLI, but each would ship a release that installed copies can't use. A wrong key is otherwise found only when someone tries to update.
2. Notes step:

   ```bash
   git fetch --force origin "+refs/tags/$GITHUB_REF_NAME:refs/tags/$GITHUB_REF_NAME"
   if [ "$(git cat-file -t "$GITHUB_REF_NAME")" = tag ]; then
     git tag -l --format='%(contents:subject)%0a%(contents:body)' "$GITHUB_REF_NAME" > notes.txt
   else
     : > notes.txt
   fi
   ```

   The fetch makes sure the tag object is there, not just the commit.
3. `scripts/release-files.sh "$GITHUB_REF_NAME" notes.txt "https://github.com/raminmz66/repo-bell/releases/download/$GITHUB_REF_NAME" release-files`
4. softprops: `files: release-files/*`, `body_path: notes.txt`, keep `generate_release_notes: true` (GitHub adds its list under our notes).

`scripts/release-files.sh <tag> <notes file> <download base URL> <out dir>`, run from the repo root, with `set -euo pipefail`:

- copies `RepoBell_<v>_amd64.deb` and `RepoBell_<v>_amd64.AppImage` from `target/release/bundle/` into the out dir;
- adds fixed-name copies `RepoBell_amd64.deb` and `RepoBell_amd64.AppImage` for the install command;
- writes `latest.json` with `jq -n`:

  ```json
  {
    "version": "0.2.0",
    "notes": "<notes file contents>",
    "pub_date": "2026-11-20T10:00:00Z",
    "platforms": {
      "linux-x86_64-deb": { "url": "<base>/RepoBell_0.2.0_amd64.deb", "signature": "<contents of the .deb.sig>" },
      "linux-x86_64-appimage": { "url": "<base>/RepoBell_0.2.0_amd64.AppImage", "signature": "<contents of the .AppImage.sig>" }
    }
  }
  ```

  The `.sig` files hold the base64 text the plugin expects (`sign_file` writes base64 of the minisign signature; the plugin decodes base64 first). `pub_date` is RFC 3339 (`date -u +%Y-%m-%dT%H:%M:%SZ`). No `linux-x86_64` key, so a build that doesn't know its type never gets a package.

The `.sig` files are not uploaded: their contents are already in `latest.json`.

### Install command for the README

Checked in the Ubuntu 22.04.5 desktop manifest: `wget` is installed by default and `curl` is not. `releases/latest/download/<name>` needs a fixed file name, so the release carries the fixed-name copies.

```bash
wget -qO /tmp/RepoBell.deb https://github.com/raminmz66/repo-bell/releases/latest/download/RepoBell_amd64.deb && sudo apt install -y /tmp/RepoBell.deb
```

`apt` pulls the dependencies (`git`, `zenity`, …). Then RepoBell is in the app menu.

AppImage, for people who don't want a package:

```bash
mkdir -p ~/Applications && wget -qO ~/Applications/RepoBell.AppImage https://github.com/raminmz66/repo-bell/releases/latest/download/RepoBell_amd64.AppImage && chmod +x ~/Applications/RepoBell.AppImage && ~/Applications/RepoBell.AppImage
```

It updates itself in place, so it needs a folder the user can write to. It needs no `libfuse2`: our AppImage uses the static type2 runtime (`--appimage-version` prints `AppImage/type2-runtime`), and on 2026-11-17 it mounted on this machine, which has only `fuse3`, like a stock 22.04.

## Signing key

The key is the stamp that proves an update comes from us. Installed copies trust only the public half, which is in `tauri.conf.json`.

- Claude makes it in the build phase, with Ramin's OK: `npm run tauri signer generate -- --ci -w ~/.tauri/repobell.key`. No password (Claude's call): GitHub secrets and a password manager already encrypt it, and a password is a second thing that can be lost. CI then needs only `TAURI_SIGNING_PRIVATE_KEY`, and the workflow passes an empty `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- The private key is never printed. Claude stores it with `gh secret set TAURI_SIGNING_PRIVATE_KEY -R raminmz66/repo-bell < ~/.tauri/repobell.key`, with Ramin's OK.
- Backup, Ramin only: open `~/.tauri/repobell.key` in a text editor, paste it into a new secure note "RepoBell update key" in your password manager, check it's saved, then delete the file.
- If the key is lost, every installed RepoBell can never update itself again. Everyone has to reinstall by hand with the install command, and that release has to carry a new public key. If the key leaks, someone who also gets control of the GitHub release could ship a fake update. Then make a new key, release with it at once, and ask everyone to reinstall.

## Edge cases

| Case | What happens |
|---|---|
| Offline at login | The start check fails silently and runs again 1 hour later. |
| User presses Check for updates while offline | "Can't reach GitHub to check for updates" and Try again. |
| A repo action runs when the user opens About | Install is disabled with "Wait for the repo action to finish." It turns on when `actions_running` goes back to 0. |
| User clicks Update on a row during the download or install | The row shows "RepoBell is installing an update and will restart in a moment." Nothing runs. |
| A check runs when the install finishes | RepoBell restarts anyway. Fetches change nothing that matters; the next start checks again. |
| Closing the first password window | A second one (zenity) shows. Closing it too gives "Update not installed. Nothing was changed." |
| The download is cut off | "The download stopped. Check your connection and try again." `download` never returns half a file. |
| `latest.json` edited to say 0.3.0 with the 0.2.0 file | The signed version doesn't match, so "didn't pass the safety check". |
| AppImage in a folder the user can't write to | The install fails and the plugin puts the old file back. "Update not installed." |
| Release without `latest.json`, or without our package | The check fails. Silent when automatic, "Can't reach GitHub" when manual. |
| A future release adds a .deb dependency | `dpkg -i` doesn't install dependencies, so the in-app update fails for people who lack it. Avoid; if needed, the release notes say to reinstall with the install command. |
| `tauri dev` or a `cargo` build | `can_update` is false: version shown, no check, no button. |
| Window hidden when an update is found | No notification. The footer and the tray line show it. |

## Keyboard and screen reader

- The footer button is a real `<button>`; its name is its text.
- Opening About from the footer or the tray moves focus to the About heading.
- The download bar is `role="progressbar"` with `aria-label="Download progress"`, `aria-valuemin="0"`, `aria-valuemax` and `aria-valuenow` in MB, and `aria-valuetext="4 of 9 MB"`. With no total it has no `aria-valuemax`.
- The three error lines are `role="alert"`, so a user who pressed a button hears the result. Automatic checks never set them.
- The disabled Install is tied to its reason with `aria-describedby`.
- The bar width eases over 0.15s, with no transition under `prefers-reduced-motion: reduce`.

## Tests

Rust (`updates.rs` unit tests, the style of `watcher.rs` tests):

- Timer wait: setup not done, update known, never tried, worked 3 h ago (21 h left), failed 10 min ago (50 min left).
- Error mapping: one case per row of the table.
- Check result: a failed automatic check keeps the previous status and sets no error; a failed manual one sets `Offline`; `Ok(None)` sets `UpToDate` and `checked_at`.
- MB step: publishes at 0, 1, 2 MB, not between.
- Install guard: refused while `actions_running > 0`; `begin_action` refused while `Downloading` or `Installing`; allowed again after a failed install.
- Tray: `install` set gives the `install_update` item text "Install RepoBell 0.2.0…"; `None` gives no item. Existing tray tests keep passing.
- `can_update` is false in tests (`bundle_type()` is `None` there), and the scheduler returns at once.

Svelte (Vitest and Testing Library):

- `AboutSection.test.ts`: one test per state row (status text and button), "RepoBell 0.1.0" always shown, notes shown only when not empty and split into items, Install disabled with the reason when `busy`, Try again calls `check_for_updates` for `offline` and `install_update` for the others, links call `open_link` with `source` and `issues`, no button and no check for `can_update: false`.
- `Footer.test.ts`: no update button without an update; "RepoBell 0.2.0 is ready" with one, and clicking it calls `onOpenAbout`. Update the existing test that expects exactly one button.
- `App.test.ts`: clicking the footer button opens Settings with About focused; an `open-about` event does the same, also when Settings is already open; no footer button during the welcome wizard.
- `util` test for `checkedAt`: today, yesterday, older.
- `makeState` in `test-helpers.ts` gets a default `update` (idle, `can_update: true`, version 0.1.0).

End-to-end before release, on Ramin's machine, with no code change (only `--config` overrides). Claude's call; it costs two local builds (about 10 minutes) and one password entry by Ramin:

1. Make a throwaway key: `npm run tauri signer generate -- --ci -w /tmp/rb-test.key`.
2. Build "old" 0.1.0 with `--config '{"plugins":{"updater":{"pubkey":"<test pub>","endpoints":["http://127.0.0.1:8765/latest.json"],"dangerousInsecureTransportProtocol":true}}}'`. Copy its .deb and AppImage aside.
3. Build "new" 0.1.1 the same way plus `"version":"0.1.1"` and `"bundle":{"createUpdaterArtifacts":true}`, with `TAURI_SIGNING_PRIVATE_KEY=/tmp/rb-test.key`.
4. Run `scripts/release-files.sh v0.1.1 notes.txt http://127.0.0.1:8765 /tmp/rb-serve` and serve that folder with `python3 -m http.server 8765`.
5. AppImage (Claude): run the old AppImage from a temp folder with its own `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_RUNTIME_DIR` (all `mktemp -d`), so Ramin's running RepoBell and its data are untouched. The runtime dir matters: at start RepoBell deletes `$XDG_RUNTIME_DIR/repobell-tray`, which holds the running copy's tray icons. Keep `DBUS_SESSION_BUS_ADDRESS` set, so the test copy's tray still reaches the session. Finish the wizard with "Start at login" off. Expect the footer line, the tray line, the notes, the download, the restart, and "RepoBell 0.1.1" in About.
6. Bad signature (Claude): append one byte to the served AppImage, restart the old one, Install. Expect "didn't pass the safety check" and the old file unchanged.
7. Offline (Claude): stop the server, press Check for updates. Expect "Can't reach GitHub…". Restart with the server still stopped and confirm no error shows.
8. .deb (Ramin, it replaces the installed RepoBell): install the old .deb, Install, close the password window twice. Expect "Update not installed." Then Try again, enter the password. Expect the restart on 0.1.1. Reinstall the normal build afterwards.
9. Busy: start an Update on a repo with unsaved changes, then open About. Expect Install disabled with the reason. Also, while an update downloads, click Update on a repo row: expect "RepoBell is installing an update and will restart in a moment." and nothing runs.

After go-live, one real smoke test before telling teammates (see Go-live, step 9). It costs one extra release (about 15 minutes of CI and a version number). It is the only test of the real key, the real secret and the GitHub URLs.

Before finishing, the builder takes screenshots at 420×640 of About (up to date, ready with notes, downloading) and the footer line.

## Go-live steps

In order. "Ramin" marks steps only Ramin can do; "OK" marks steps Claude does after Ramin says yes.

1. Claude (OK): generate the signing key, put the public key in `tauri.conf.json`, set the `TAURI_SIGNING_PRIVATE_KEY` secret.
2. Ramin: back up the key to the password manager and delete the file.
3. Claude: build, test, review and merge this feature. Reinstall as usual.
4. Claude and Ramin: the end-to-end test above (Ramin does step 8).
5. Claude (OK): make an annotated tag `v0.1.0` with first-release notes and push it. The release workflow runs.
6. Claude: check the release has both packages, both fixed-name copies and `latest.json`, and that `latest.json` parses and names `0.1.0` (`gh release download v0.1.0 -p latest.json`, as the repo is still private).
7. Ramin: make the repo public (GitHub, Settings, Danger zone).
8. Claude: check from a fresh shell, without `gh`, that `latest.json` and `RepoBell_amd64.deb` download through the `releases/latest/download/` URLs. Ramin installs v0.1.0 with the README command.
9. Claude (OK): release `v0.1.1` (version bump only, notes "Test release for the updater"). Ramin's v0.1.0 .deb and an AppImage v0.1.0 both update from inside the app.
10. Ramin: send teammates the install command.

## Out of scope

- A desktop notification for a new version, a "check automatically" switch, installing without a click.
- rpm, Snap, Flatpak, an apt repository, macOS and Windows.
- Beta channels, downgrades, rollback, delta updates.
- A key rotation plan beyond the note above.
- Our own .deb installer (to avoid the second password window).
- Uploading `.sig` files, a `CHANGELOG.md`.
- Showing GitHub's generated notes in the app.

## Left to the builder

- Exact spacing within DESIGN.md tokens and the About card layout at 420 px.
- The bell icon in the About card (reuse an SVG the window already has).
- The shape of `ActionGuard` and where the timer's pure function lives, as long as the tests pass.
- Final wording of the four new lines and the notes in the first release. Text the user reads goes through the `humanizer` skill.

## Decision text for docs/decisions.md

Add when built:

> ## About and updates
>
> Decided 2026-11-17. Full design: `docs/superpowers/specs/2026-11-17-about-and-updates-design.md`.
>
> Settings ends with an About section: the version, one status line, one button, the new version's notes, and links to the source and to GitHub issues. When a new version is ready, the main window footer says "RepoBell 0.2.0 is ready" in blue with a grey dot (blue dots mean team news), and the tray menu gets "Install RepoBell 0.2.0…" above Quit. Both open About. Nothing installs without a click, so the password prompt is never a surprise.
>
> RepoBell checks GitHub at start and every 24 hours, not during the first-run welcome. A failed automatic check stays silent and tries again an hour later. Errors show only after the user pressed a button. There is no switch to turn checks off. Install is off while Update, Undo, a branch delete or Update main runs, so a restart never cuts one in half.
>
> Updates use Tauri's updater plugin, for the .deb and the AppImage. Each package is signed with a key only the release workflow has, and RepoBell refuses a package whose signature or version doesn't match. If the key is lost, installed copies can never update again, so it is backed up in Ramin's password manager. A copy built from source shows its version and doesn't update itself.

Replace the "Releases" section with:

> ## Releases
>
> GitHub Actions runs the tests and checks on every push and pull request. Pushing a version tag such as `v0.1.0` builds and signs the .deb and AppImage and attaches them to a GitHub release, with fixed-name copies for the install command and a `latest.json` that RepoBell reads for updates. The notes come from the annotated tag's message. Anyone can download them without a GitHub account.

## Docs to update when built

- `docs/decisions.md`: the section above and the new "Releases" text.
- `README.md`: the install command, the AppImage lines, and one line on updates ("RepoBell checks for a new version once a day; Settings, About installs it").
- `CONTRIBUTING.md` 4.3: tag with `git tag -a vX.Y.Z -m "…"` for notes (a plain tag gives none); local builds don't sign and don't need the key; the release needs the `TAURI_SIGNING_PRIVATE_KEY` secret; don't add a .deb dependency without telling users to reinstall.
- `DESIGN.md`: the footer update button (blue text, grey dot) and the About card.
- `ROADMAP.md`: the new item with its steps, and the current step.
