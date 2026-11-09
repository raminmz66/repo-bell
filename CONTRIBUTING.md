# Contributing to RepoBell

Thank you for your interest in contributing to **RepoBell**! RepoBell is an ambient Git repository watcher and "what's new" feed designed specifically for developers and designers working together on Ubuntu.

---

## 1. Project Philosophy & Design Principles

- **Ambient & Unobtrusive:** RepoBell sits quietly in the top bar. It never interrupts unless there is genuinely actionable news from your teammates.
- **Safe & Non-Destructive:** RepoBell never destroys local work. Updates use fast-forward pulls or stash-and-pop with automated rollbacks if a conflict occurs.
- **Plain English:** We prioritize clear, human terms ("the main version", "Behind by 4", "Has unsaved changes") over obscure Git terminology.
- **Lightweight & Fast:** Low CPU and memory footprint across long-running desktop sessions.

---

## 2. Architecture Overview

The codebase is organized as a Cargo workspace with a frontend Vite/Svelte client:

```text
repo-bell/
├── crates/
│   └── core/                # Pure Rust engine: discovery, status, fetch,
│                            # feed generation, safe update, undo, and error classification.
├── app/
│   ├── src-tauri/           # Tauri v2 desktop harness (Rust):
│   │   ├── src/watcher.rs   # Background periodic check loop & state aggregator
│   │   ├── src/actions.rs   # IPC command handlers
│   │   ├── src/notify.rs    # Native Linux notification engine (notify-send)
│   │   ├── src/autostart.rs # Login autostart configuration generator
│   │   └── src/store.rs     # Atomic settings & seen state persistence
│   └── src/                 # Svelte 5 frontend (TypeScript + Vite):
│       ├── lib/RepoCard.svelte     # Repository feed cards with update/undo flows
│       ├── lib/feedGrouping.ts     # Groups commits chronologically by date and author
│       ├── lib/SettingsView.svelte # Watcher preferences & editor configuration
│       └── lib/WelcomeView.svelte  # 3-step first-run onboarding wizard
├── docs/                    # Design mockups, decisions (ADR), and registry verification
├── SPEC.md                  # Detailed architectural specification
└── ROADMAP.md               # Milestones and release tracking
```

---

## 3. Prerequisites & Development Setup

### 3.1 System Dependencies (Ubuntu 22.04+)
RepoBell uses WebKitGTK and Ayatana AppIndicator. Install required development libraries:

```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  libxdo-dev \
  libssl-dev \
  librsvg2-dev \
  build-essential \
  curl \
  wget \
  file
```

### 3.2 Toolchains
- **Rust:** Stable 1.80+ (`rustup default stable`)
- **Node.js:** v18+ with **npm** (do not use yarn or pnpm)

### 3.3 Initial Setup
```bash
git clone https://github.com/raminmz66/repo-bell.git
cd repo-bell/app
npm ci
```

---

## 4. Development & Testing Workflow

### 4.1 Running in Development Mode
Launch Vite dev server and the Tauri desktop app:
```bash
cd app
npm run tauri dev
```

### 4.2 Checks before a pull request
CI runs the same checks:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd app
npm run check   # svelte-check
npm test
```

### 4.3 Packaging and releases
To build the `.deb` and AppImage locally:
```bash
cd app
npm run tauri build
```
The packages land in `target/release/bundle/`.

To publish a release:
1. Set the new version in `app/src-tauri/tauri.conf.json`, `crates/core/Cargo.toml`, `app/src-tauri/Cargo.toml` and `app/package.json`.
2. Commit and merge to `main`.
3. Tag it and push the tag:
   ```bash
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

The release workflow checks that the tag matches `tauri.conf.json`, builds the packages and attaches them to a GitHub release.

---

## 5. Contribution Guidelines

### 5.1 Branching Strategy
- Never commit directly to `main`.
- Create a focused feature branch for your work:
  - `feat/<feature-name>` for new capabilities
  - `fix/<bug-name>` for bug fixes
  - `docs/<doc-name>` for documentation updates

### 5.2 Commit Message Conventions
We adhere strictly to [Conventional Commits](https://www.conventionalcommits.org/):
- `feat: add desktop notification throttling`
- `fix: prevent race condition on atomic seen file write`
- `test: add unit tests for date grouping in feed`
- `docs: update packaging instructions in README`
- `refactor: simplify status check error classification`

### 5.3 Code Style
- **Rust:** Format with `cargo fmt` and keep `cargo clippy -- -D warnings` clean. Avoid `.unwrap()` in library code (`crates/core`).
- **Svelte / TypeScript:** Strict TypeScript typing enabled (`"strict": true`). Use standard Svelte 5 idioms and clean accessible HTML (`aria-*` labels).

---

## 6. Filing Issues & Questions

If you encounter a bug or have an idea for improvement, please open an issue on GitHub describing:
1. Steps to reproduce
2. Expected vs. actual behavior
3. Linux distribution and desktop environment (e.g., Ubuntu 24.04 GNOME X11/Wayland)
