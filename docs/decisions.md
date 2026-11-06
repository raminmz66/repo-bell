# RepoBell decisions

RepoBell is a free, open-source tray app for Ubuntu (22.04 first). It watches the Git repos on your machine and shows what changed since you last looked. It is for developers and designers on the same team.

## Scope

RepoBell watches repos and shows a "what's new" feed: new commits on main, who pushed them and which files changed. It is not a full Git client, because GitKraken and GitHub Desktop already cover that.

The closest existing tool is Time2Pull. It warns you when a remote changes, but it uses old PyQt5 code, has no Linux packages and looks unmaintained.

## Stack

RepoBell uses Tauri, with a Rust backend and a web UI. The app runs all day, so low memory use matters more than build speed. It calls Git through the command line rather than a library, which is simpler and behaves the same as the user's own Git.

## Name

The name is RepoBell. GitGlance and RepoLens were already used by several Git tools. No project called RepoBell turned up in web searches. The nearest name is RepoBee, a command-line tool for teachers. Before release, check npm, crates.io, a domain and the GitHub org.

## v1 features

- Find repos in folders the user chooses.
- Show a status per repo: behind, ahead, unsaved changes.
- Show the "what's new" feed for main since the last look.
- Send desktop notifications.
- Use plain words that designers understand, such as "the main version", "Behind by 4" and "Has unsaved changes". Status labels keep the short Git terms ("Behind by 4", "Ahead by 1"), and hovering a label explains it ("You have 1 change the team doesn't have yet").
- Add an Update button (see below), with Undo.

Errors show at two levels. When every check fails (usually no internet), the whole window says "Can't check your repos". When one repo fails, only its card shows "Can't check", with a plain reason ("Git can't log in to the server", "Folder not found") and one action ("Try again" or "Remove").

Each feed card lists change titles, the same way for every repo. Below them, a line such as "6 files changed" opens to show the file names.

Opening the repo folder and copying a message for a developer were both dropped. Neither helps solve a conflict.

## Update button

| Case | What happens |
|---|---|
| Safe | Runs `git pull --ff-only`. |
| Unsaved changes | Asks first, then stashes, pulls and pops the stash. If the pop conflicts, it is handled as a conflict. |
| Diverged | Offers "Update and combine", which is a normal merge. If the merge fails, it aborts. |
| Conflict | Aborts and restores the previous state. Says "Nothing was changed" and lists the conflicting files, each with "Open in editor". |

RepoBell never resolves conflicts itself and never switches branches. Before acting it shows a one-line preview ("This will download 4 changes"). After an update it offers Undo, using the reflog. Undo stays on the card until the repo changes or the window closes. RepoBell checks before undoing: if the user edited files or committed since the update, Undo is greyed out with "You changed files since the update".

## Tray on Linux

Tauri does not report clicks on the tray icon on Linux, so a click cannot open a custom popup. The right-click menu is the quick view instead:

- A headline, such as "3 repos have news"
- Up to 5 repo lines with their status, then "and N more…"
- Open RepoBell, Check now, Mark all as seen, Quit

Clicking a repo line opens the main window on that repo. A number badge is likely not possible, so the icon changes instead: a bell when all is quiet, a bell with a dot when there is news, and a crossed bell when RepoBell can't check. On Linux the icon may only show once a menu is set.

## Design reviews

Mockups are HTML files in `docs/mockups/`. Each one is also published as a private claude.ai artifact so it can be opened on a phone.

Every mockup gets a senior UI/UX review: what works, what to fix, the missing states (first run, all quiet, offline, error) and the platform limits that affect the design.
