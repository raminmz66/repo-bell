# RepoBell roadmap

Current step: Phase 1, desktop notification mockup (one repo and several repos).

Decisions and their reasons are in [docs/decisions.md](docs/decisions.md). Mockups live in [docs/mockups](docs/mockups) and on the [RepoBell mockups canvas](https://claude.ai/artifact/LuGyXkKCaV5K5jS4yKEvjr) (private).

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
- [ ] Desktop notification (one repo and several repos)
- [ ] Settings and add-folder screen, including the editor choice
- [ ] First-run welcome, including the Ubuntu tray check

## Phase 2: risk checks

- [ ] Install Rust and the Tauri system packages
- [ ] Build a small Tauri tray test on Ubuntu 22.04: the icon shows, the menu text updates live, the icon can switch between states
- [ ] Decide how often to check repos without heavy network use
- [ ] Decide how to detect the user's editor

## Phase 3: build v1

- [ ] Project skeleton (Tauri app, tray, main window)
- [ ] Find repos in the chosen folders
- [ ] Status per repo: behind, ahead, unsaved changes
- [ ] "What's new" feed since the last look
- [ ] Desktop notifications
- [ ] Update button: safe, unsaved changes, diverged, conflict
- [ ] Undo after update
- [ ] Open conflicting files in the editor
- [ ] Final review for regressions

## Phase 4: v1.1 and first release

- [ ] Packages: .deb and AppImage, maybe Flatpak
- [ ] Start at login
- [ ] License, README, contributing guide
- [ ] Check the name on npm, crates.io, a domain and GitHub
- [ ] Feed grouped by person and day

## Later

- GitHub and GitLab info (open pull requests, who pushed)
- macOS and Windows
- Better conflict help
