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
- **Editor integration:** Auto-detects installed editors (VS Code, JetBrains, Sublime, Text Editor) and opens conflicting files.
- **First-run wizard:** Discovers your Git projects automatically and tests top bar indicator visibility.

---

## Installation & Setup

### Option 1: Debian Package (.deb) — Recommended for Ubuntu
Installs system-wide, integrates into the Ubuntu Application Menu, and enables desktop autostart:

```bash
sudo apt install ./target/release/bundle/deb/RepoBell_0.1.0_amd64.deb
```

Launch it from Ubuntu's application grid (search for **RepoBell**) or from the terminal:
```bash
repobell &
```

To uninstall:
```bash
sudo apt remove repo-bell
```

### Option 2: Portable AppImage (No root / sudo required)
Make executable and run anywhere:

```bash
chmod +x ./target/release/bundle/appimage/RepoBell_0.1.0_amd64.AppImage
./target/release/bundle/appimage/RepoBell_0.1.0_amd64.AppImage &
```

### Option 3: Direct Release Binary
```bash
./target/release/repobell &
```

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
