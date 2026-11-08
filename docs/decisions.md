# RepoBell decisions

RepoBell is a free, open-source tray app for Ubuntu (22.04 first). It watches the Git repos on your machine and shows what changed since you last looked. It is for developers and designers on the same team.

## Scope

RepoBell watches repos and shows a "what's new" feed: new commits on main, who pushed them and which files changed. It is not a full Git client, because GitKraken and GitHub Desktop already cover that.

The closest existing tool is Time2Pull. It warns you when a remote changes, but it uses old PyQt5 code, has no Linux packages and looks unmaintained.

## Stack

RepoBell uses Tauri, with a Rust backend and a web UI. The app runs all day, so low memory use matters more than build speed. It calls Git through the command line rather than a library, which is simpler and behaves the same as the user's own Git.

The window is written in Svelte with Vite. Svelte compiles down to small plain JavaScript, so the app stays light while it runs all day, and it keeps the many card states (Update, Undo, errors, empty states) simple to write. React was the other option; it has more contributors who know it, but it is heavier and we don't need its ecosystem for four screens.

## License and hosting

RepoBell uses the MIT license. The code lives in the private GitHub repo `raminmz66/repo-bell` until the first release.

## Name

The name is RepoBell. GitGlance and RepoLens were already used by several Git tools. No project called RepoBell turned up in web searches. The nearest name is RepoBee, a command-line tool for teachers. Before release, check npm, crates.io, a domain and the GitHub org.

## Where RepoBell keeps its data

RepoBell keeps two small JSON files in the standard Linux folders. Settings (folders, muted repos, editor, check interval, start at login) live in `~/.config/repobell/settings.json`. The "last seen" commit for each repo lives in `~/.local/state/repobell/seen.json`. Keeping them apart means settings can be copied to another machine without bringing wrong "last seen" points, and the often-changing file never touches settings. Losing `seen.json` only makes everything show as new once. Each save writes a temporary file and then renames it, so a crash never leaves a half-written file. RepoBell never writes its own data into the user's repos.

## Checking for news

RepoBell checks every 5 minutes by default. Settings offers 1, 5, 15, 30 and 60 minutes, and "Check now" is always available. Each check fetches only the main branch, and repos are checked one after another. After an error the wait doubles (5, 10, 20 minutes and so on) until a check works again. This keeps news fresh without heavy network use or battery drain.

## v1 features

- Find repos in folders the user chooses.
- Show a status per repo: behind, ahead, unsaved changes.
- Show the "what's new" feed for main since the last look.
- Send desktop notifications.
- Use plain words that designers understand, such as "the main version", "Behind by 4" and "Has unsaved changes". Status labels keep the short Git terms ("Behind by 4", "Ahead by 1"), and hovering a label explains it ("You have 1 change the team doesn't have yet").
- Add an Update button (see below), with Undo.
- Start at login, asked on the last first-run step.

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

RepoBell never resolves conflicts itself and never switches branches. Before acting it shows a one-line preview ("This will download 4 changes"). After an update it offers Undo, using the reflog. Undo stays on the card until the repo changes or the window closes. It runs `git reset --keep`, which moves the commits back but keeps unsaved changes, and Git itself refuses when an unsaved change would be overwritten. So Undo is greyed out once the user commits or updates again. If their file edits clash with the undo, it stops and changes nothing ("You changed files since the update"). Edits to other files survive the undo. This is safer than checking for edits ourselves, because Git does the check.

## First run

On first run RepoBell suggests folders before asking. It looks in common places (`~/projects`, `~/code`, `~/work`, `~/Documents`), a few levels deep, skipping folders like `node_modules`, and shows what it found ("Found 5 repos in 2 folders"). The user unticks repos or adds more folders. Then RepoBell checks that the bell can show in the top bar. If it can't, it explains how to turn on the "Ubuntu AppIndicators" extension, and the user can continue without the bell. The last step has a ticked checkbox, "Start RepoBell when I log in", so RepoBell keeps running after a reboot. Start at login moved from v1.1 into v1 for this reason.

## Settings

Settings shows one list grouped by folder. Each folder is a header with Remove, and its repos sit under it. Each repo has a notify switch and a small menu with "Stop watching", so one repo can be dropped without removing its folder.

The editor list shows the editors RepoBell finds on the computer (VS Code, JetBrains IDEs, Sublime Text, Ubuntu's text editor) plus "System default app", which covers every other editor. Picking any app or typing a custom command can come later if people ask.

RepoBell finds editors by reading the `.desktop` files that Ubuntu's app menu uses (system, user, Snap and Flatpak locations) and matching known editors. That finds apt, Snap, Flatpak and JetBrains Toolbox installs alike. It opens files through the desktop launcher, and "System default app" uses `xdg-open`.

## Notifications

RepoBell sends one notification per check, never one per change. With one repo it names the repo, the person and the first change title. With several it says how many repos have news. The user's own changes never trigger a notification. A change counts as their own when its author email matches their Git email in that repo. Own changes still show in the feed as "You added 2 changes", so a second machine explains why it is behind. Every repo sends notifications by default. Each repo has a notify switch in settings. A muted repo still shows its news in the window and the tray menu, but it sends no notifications. Notifications have no buttons: clicking one opens RepoBell, on the repo when there is only one. This keeps the "preview before acting" rule, and Tauri's notification plugin may not support buttons on Linux.

## Tray on Linux

Tauri does not report clicks on the tray icon on Linux, so a click cannot open a custom popup. The right-click menu is the quick view instead:

- A headline, such as "3 repos have news"
- Up to 5 repo lines with their status, then "and N more…"
- Open RepoBell, Check now, Mark all as seen, Quit

Clicking a repo line opens the main window on that repo. A number badge is likely not possible, so the icon changes instead: a bell when all is quiet, a bell with a dot when there is news, and a crossed bell when RepoBell can't check. On Linux the icon may only show once a menu is set.

A tray test on Ubuntu 22.04 (GNOME on X11, Tauri 2.12.5) confirmed this plan on 2026-11-06. The icon shows and switches between three states. A left click opens the menu. Menu text updates live, even while the menu is open. "Open RepoBell" opens the window. No tray icon events reached the app, as expected. Build notes from that test: use `TrayIconBuilder` with `show_menu_on_left_click(true)`, change icons with `tray.set_icon`, change text with `MenuItem::set_text`, and make every icon PNG 32-bit RGBA, because `generate_context!` rejects other PNGs.

## Design reviews

Mockups are HTML files in `docs/mockups/`. Each one is also published as a private claude.ai artifact so it can be opened on a phone.

Every mockup gets a senior UI/UX review: what works, what to fix, the missing states (first run, all quiet, offline, error) and the platform limits that affect the design.
