# RepoBell

An ambient tray app for Ubuntu that tells you when your team changes the Git repos on your computer.

RepoBell sits quietly in your Ubuntu top bar, watches your Git repos, and tells you when your team pushed changes without interrupting your flow.

---

## What It Does

- **Ambient top bar watcher:** System tray icon with dynamic states: all quiet, news waiting, or connection error.
- **"What's new" feed:** Shows new commits on the main branch grouped chronologically by person and day.
- **Smart desktop notifications:** Sends one summary notification per check cycle when teammates push changes, never notifying for your own commits.
- **Safe 1-click updates:** Updates cleanly with fast-forward or stash-and-pop, aborting safely if conflicts would arise.
- **Instant Undo:** Reverts updates cleanly with reflog protection if you need to roll back.
- **Old branch cleanup:** Offers to delete local branches your team already deleted on the server, with Undo. It works best when GitHub's "Automatically delete head branches" setting is on.
- **Editor integration:** Auto-detects installed editors (VS Code, JetBrains, Sublime, Text Editor) and opens conflicting files.
- **First-run wizard:** Discovers your Git projects automatically and tests top bar indicator visibility.

---

## Install

```bash
wget -qO /tmp/RepoBell.deb https://github.com/raminmz66/repo-bell/releases/latest/download/RepoBell_amd64.deb && sudo apt install -y /tmp/RepoBell.deb
```

Then open RepoBell from the app menu. RepoBell checks for a new version once a day; Settings, About installs it.

Or use the AppImage, if you don't want a package:

```bash
mkdir -p ~/Applications && wget -qO ~/Applications/RepoBell.AppImage https://github.com/raminmz66/repo-bell/releases/latest/download/RepoBell_amd64.AppImage && chmod +x ~/Applications/RepoBell.AppImage && ~/Applications/RepoBell.AppImage
```

To remove the package: `sudo apt remove repo-bell`.

## Build from source

See [CONTRIBUTING.md](CONTRIBUTING.md) for the system packages. Then:

```bash
cd app
npm ci
npm run tauri build
```

The packages land in `target/release/bundle/`.

---

## First-Run Setup Wizard

When you first launch RepoBell, a 3-step setup wizard will guide you:
1. **Find Repos:** Automatically scans common locations (`~/projects`, `~/code`, `~/Documents`, etc.) and shows the repos it found.
2. **Top Bar Check:** Confirms the bell icon is visible in your Ubuntu top bar (or advises how to enable the Ubuntu AppIndicators extension if hidden).
3. **Start at Login:** Offers a single checkbox to keep RepoBell running after you reboot (`~/.config/autostart/repobell.desktop`).

---

## Development & Documentation

- [Contributing Guide](CONTRIBUTING.md) — Prerequisites, test running, and development server.
- [Specification](SPEC.md) — Architecture and IPC contracts.
- [Roadmap](ROADMAP.md) — Progress and post-v1 items.
- [Decisions](docs/decisions.md) — Architecture decision records.

---

## License

[MIT](LICENSE)
