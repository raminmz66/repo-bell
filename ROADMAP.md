# RepoBell roadmap

Current step: Getting ready for teammates (2026-11-17). The repo is going public, with commit emails rewritten to Ramin's GitHub private email. Now building About and in-app updates (spec and plan in docs/superpowers). Next: release v0.1.0, then make the repo public.

Decisions and their reasons are in [docs/decisions.md](docs/decisions.md). Mockups live in [docs/mockups](docs/mockups).

## Phase 0: research and decisions

- [x] Research Git pain points and existing tools
- [x] Choose scope: repo watcher plus a "what's new" feed
- [x] Choose stack: Tauri
- [x] Choose name: RepoBell
- [x] Choose v1 features
- [x] Write user journeys
- [x] Confirm the tray limit on Linux (Tauri reports no clicks on the tray icon)
- [x] Set up the repo, this roadmap and the decisions file

## Phase 1: finish the design

Each screen gets a senior UI/UX review before it is marked done.

- [x] Decide the mockup format (HTML in docs/mockups, published as an artifact)
- [x] Redo the lost mockups: main window, Update states, empty states, tray menu
- [x] UX review of the redone mockups, then apply the accepted fixes
- [x] Desktop notification (one repo and several repos)
- [x] Settings and add-folder screen, including the editor choice and the per-repo Mute switch
- [x] First-run welcome, including the Ubuntu tray check

## Phase 2: risk checks

- [x] Install Rust (rustup, in the home folder)
- [x] Install the Tauri system packages (Ramin runs it, needs sudo): `sudo apt update && sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`
- [x] Build a small Tauri tray test on Ubuntu 22.04: the icon shows, the menu text updates live, the icon can switch between states
- [x] Decide how often to check repos without heavy network use
- [x] Decide how to detect the user's editor

## Phase 3: build v1

- [x] Project skeleton (Tauri app, tray, main window, settings and "last seen" files)
- [x] Find repos in the chosen folders (core: discover)
- [x] Status per repo: behind, ahead, unsaved changes (core: status)
- [x] "What's new" feed since the last look (core: fetch, feed)
- [x] Plain error reasons for the repo card (core: errors)
- [x] Background checks on a timer, tray icon states and tray menu
- [x] Main window: repo cards, feed, Update and Undo
- [x] Settings screen
- [x] Desktop notifications
- [x] Update button: safe, unsaved changes, diverged, conflict (core: update)
- [x] Undo after update (core: undo)
- [x] Open conflicting files in the editor
- [x] First-run flow: suggested folders, tray check, start at login
- [x] Final review for regressions

## Phase 4: v1.1 and first release

- [x] Packages: .deb and AppImage
- [x] License (MIT) and a short README
- [x] Contributing guide
- [x] Check the name on npm, crates.io, a domain and GitHub
- [x] Feed grouped by person and day
- [x] Review fixes (41 findings from the 2026-11-09 review)
- [x] CI and release workflow (CI passed on GitHub on 2026-11-09; release workflow runs on the first tag)

## Phase 5: branch cleanup

- [x] Design approved (2026-11-10)
- [x] Build plan
- [x] Build, test and review
- [x] Install and hand test (delete and Undo tried by Ramin on a test repo, 2026-11-17)
- [ ] Follow-ups: remove the duplicate UI test; treat an unreadable worktree state file as checked out

## Fixes

- [x] News clears after Update or a pull, so "Mark all as seen" isn't needed for changes you already have (2026-11-14)
- [x] Main window redesign: grouped rows instead of cards (2026-11-14)
  - [x] Design (`docs/superpowers/specs/2026-11-14-grouped-rows-design.md`)
  - [x] Plan
  - [x] Build
  - [x] Hand check in the running app, merge and reinstall
  - [x] Tray jump now scrolls to the row (WebKitGTK ignored smooth scrolling)
- [x] Check progress and 4 repos at once (2026-11-17)
  - [x] Design (`docs/superpowers/specs/2026-11-17-check-progress-design.md`)
  - [x] Plan
  - [x] Build
  - [x] Merge and reinstall
  - [x] Hand test by Ramin
- [ ] Known small issue: Undo after a background check can send a second notification for changes you already saw. Left as is (rare, harmless).

## Later

- Offline: keep the last list visible with a "Can't reach the server" banner instead of hiding everything.
- GitHub and GitLab info (open pull requests, who pushed)
- macOS and Windows
- Better conflict help
