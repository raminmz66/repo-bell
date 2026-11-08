# RepoBell Phase 4: Packaging & Release Preparation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete all Phase 4 release preparation tasks: feed grouping by person and day, Linux packaging (.deb & AppImage), contributing guide, registry name verification, and roadmap finalization.

**Architecture:** Svelte 5 frontend component enhancements (`feedGrouping.ts`, `RepoCard.svelte`), Tauri v2 configuration (`tauri.conf.json`), documentation (`CONTRIBUTING.md`, `docs/name-check.md`), and Debian packaging verification.

**Tech Stack:** Rust, Svelte 5, TypeScript, Vite, Tauri v2 bundler.

**Spec:** [SPEC.md](SPEC.md)

---

### Task 1: Feed Grouped by Person and Day (TypeScript & Svelte)
**Files:**
- Create: `app/src/lib/feedGrouping.ts`
- Create: `app/src/lib/feedGrouping.test.ts`
- Modify: `app/src/lib/types.ts`
- Modify: `app/src/lib/RepoCard.svelte`

**Interfaces:**
- Input: `changes: ChangeView[]`
- Output: `DayGroup[]` rendered in `RepoCard.svelte` with day headers (Today, Yesterday, Date), author headers ("Sarah Chen", "You"), and commit items.

- [ ] **Step 1: Write unit tests for `groupChangesByPersonAndDay` in `feedGrouping.test.ts`**
- [ ] **Step 2: Implement `groupChangesByPersonAndDay` in `feedGrouping.ts` and add types to `types.ts`**
- [ ] **Step 3: Update `RepoCard.svelte` to render grouped feed layout**
- [ ] **Step 4: Run tests (`npm run test` or vitest / build) and verify visually**
- [ ] **Step 5: Commit** (`feat: implement feed grouped by person and day`)

---

### Task 2: Registry & Name Verification
**Files:**
- Create: `docs/name-check.md`
- Modify: `docs/decisions.md`

**Actions:**
- Verify name availability for `repobell` on npm, crates.io, GitHub, and domain registries.
- Record exact commands and query outputs in `docs/name-check.md`.
- Update `docs/decisions.md` with findings and recommendations.

- [ ] **Step 1: Query npm registry for `repobell` and `repo-bell`**
- [ ] **Step 2: Query crates.io API for `repobell` and `repo-bell`**
- [ ] **Step 3: Check GitHub repository / org availability**
- [ ] **Step 4: Check DNS / domain availability**
- [ ] **Step 5: Record findings in `docs/name-check.md` and update `docs/decisions.md`**
- [ ] **Step 6: Commit** (`docs: verify repobell name availability on registries and domains`)

---

### Task 3: Contributing Guide (`CONTRIBUTING.md`)
**Files:**
- Create: `CONTRIBUTING.md`

**Content:**
- Prerequisites: Ubuntu 22.04+ system dependencies, Rust, Node.js.
- Architecture explanation: core crate vs desktop app vs UI.
- Development workflow: running dev mode, running tests, typechecking.
- Conventional commits format, branch naming, and PR guidelines.

- [ ] **Step 1: Write comprehensive `CONTRIBUTING.md`**
- [ ] **Step 2: Verify all paths and commands work on the machine**
- [ ] **Step 3: Commit** (`docs: add comprehensive contributing guide`)

---

### Task 4: Linux Packaging (.deb & AppImage)
**Files:**
- Modify: `app/src-tauri/tauri.conf.json`

**Actions:**
- Configure Tauri bundler with category, description, and debian dependencies (`libwebkit2gtk-4.1-0`, `libayatana-appindicator3-1`, `libxdo3`).
- Run `npm run tauri build -- -b deb` to build `.deb`.
- Check if AppImage can be built via `npm run tauri build -- -b appimage`.
- Verify `.deb` file with `dpkg-deb --info` and `dpkg-deb --contents`.

- [ ] **Step 1: Update `tauri.conf.json` bundle configuration**
- [ ] **Step 2: Build Debian package (`.deb`)**
- [ ] **Step 3: Verify Debian package contents and dependency metadata**
- [ ] **Step 4: Test AppImage target**
- [ ] **Step 5: Commit** (`feat: configure and build Linux packages`)

---

### Task 5: Final Review, Roadmap Update & Branch Integration
**Actions:**
- Run full test suite: `cargo test` and `cd app && npm run build`.
- Update `ROADMAP.md` checking off all Phase 4 items.
- Merge `feat/phase4-release-prep` into `main`.
- Push `origin/main`.
- Delete local `feat/phase4-release-prep` branch.
- Report completion to user with verification evidence.

- [ ] **Step 1: Run complete verification suite (`cargo test`, `npm run build`, `svelte-check`)**
- [ ] **Step 2: Update `ROADMAP.md` marking Phase 4 complete**
- [ ] **Step 3: Commit** (`chore: update roadmap marking Phase 4 complete`)
- [ ] **Step 4: Merge to `main`, push to `origin/main`, delete feature branch**
