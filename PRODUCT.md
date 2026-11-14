# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

The UI is a web view inside a Tauri desktop app for Ubuntu 22.04+ (GNOME, X11). The main window is 420×640 and opens from the tray menu.

## Users

Primary: developers who watch 10 to 30 Git repos on their machine, often with several of them on side branches or worktrees. They open the window from the tray between tasks to answer "what changed, and does anything need me?", act on one or two repos, and close it.

Secondary: designers and other teammates who watch 2 to 5 repos and don't know Git well. They mostly read what changed. A short list must stay easy for them, and every label stays in plain words.

## Product Purpose

RepoBell sits in the Ubuntu top bar, checks the watched repos in the background, and tells you when your team changed the main version since you last looked. The window shows the news per repo, lets you update safely in one click with Undo, and offers to clean up old branches. Success: a glance tells you which repos have news or need action, without opening a Git client.

## Positioning

An ambient watcher, not a Git client. It never switches branches, never resolves conflicts, and explains every action in plain words before it runs. GitKraken and GitHub Desktop cover full Git work; Time2Pull is the nearest watcher and looks unmaintained.

## Operating Context

- Opened from the tray menu or a desktop notification, often to look at one repo (the window scrolls to it and flashes it).
- Background check every 5 minutes by default; "Check now" is always available.
- Repos can be muted (no notifications) or ignored (not watched) in Settings.
- The Ubuntu tray can't report clicks on the icon, so the tray menu is the only way in.

## Capabilities and Constraints

- Per repo: news count since last look, behind / ahead, unsaved changes, current branch, old branches the server deleted, errors with a plain reason.
- Actions: Update (with a one-line preview first), Update main when on a side branch, Undo, delete old branches with Undo, open conflicting or changed files in the editor, Mark all as seen.
- News means teammates' changes on main since you last looked. Your own changes and changes you already pulled are not news. "Behind by N" is a separate, quiet status.
- Status labels keep short Git terms ("Behind by 4", "Ahead by 1") with a hover that explains them. Commit SHAs are never shown.
- Light theme only today. Built with Svelte 5 and Vite.

## Brand Commitments

- Name: RepoBell. Tray icons: bell (quiet), bell with dot (news), crossed bell (can't check). The window uses the same bell icons.
- Voice: short, plain English that designers understand ("the main version", "Has unsaved changes"). No Git jargon beyond the status labels.

## Evidence on Hand

- Decisions and their reasons: `docs/decisions.md`. Mockups: `docs/mockups/`.
- No users outside the team yet, no testimonials, no usage numbers. Don't invent them.

## Product Principles

1. A glance first: the window must answer "what's new and what needs me" before any scrolling.
2. Safe and reversible: preview before acting, Undo after, never touch work Git can't put back.
3. Plain words over Git words.
4. Quiet by default: things that need nothing from you step back.

## Accessibility & Inclusion

Keyboard use works across the window (focus returns to the triggering button after dialogs). Honors reduced motion. Hover text explains status labels.
