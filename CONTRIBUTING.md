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
│   └── repobell_core/       # Pure Rust engine: discovery, status, fetch,
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
cd repo-bell
cd app
npm install
```

---

## 4. Development & Testing Workflow

### 4.1 Running in Development Mode
Launch Vite dev server and the Tauri desktop app:
```bash
cd app
npm run tauri dev
```

### 4.2 Running Tests
Before opening a pull request, run all unit and integration tests:

1. **Rust Test Suite (core engine & Tauri backend):**
   ```bash
   cargo test
   ```

2. **Frontend Unit Tests (Vitest):**
   ```bash
   cd app
   npm run test
   ```

3. **Frontend Typecheck & Production Build:**
   ```bash
   cd app
   npm run build
   ```

### 4.3 Packaging Release Builds
To build a standalone Debian package (`.deb`):
```bash
cd app
npx tauri build -b deb
```
The resulting `.deb` package will be located in `target/release/bundle/deb/`.

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
- **Rust:** Format with `rustfmt` and ensure clean `cargo clippy` runs. Avoid `.unwrap()` in library code (`repobell_core`).
- **Svelte / TypeScript:** Strict TypeScript typing enabled (`"strict": true`). Use standard Svelte 5 idioms and clean accessible HTML (`aria-*` labels).

---

## 6. Filing Issues & Questions

If you encounter a bug or have an idea for improvement, please open an issue on GitHub describing:
1. Steps to reproduce
2. Expected vs. actual behavior
3. Linux distribution and desktop environment (e.g., Ubuntu 24.04 GNOME X11/Wayland)
