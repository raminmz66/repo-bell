# Branch cleanup design

Date: 2026-11-10. Status: draft, waiting for Ramin's approval.

## Why

Local branches pile up. A developer opens a branch, pushes it, the pull request gets merged on GitHub, and the local branch stays on the laptop forever. After a few months `git branch` lists dozens of dead names, and nobody remembers which ones still matter. Deleting them by hand means checking each one, and the common "Squash and merge" on GitHub makes Git think they are not merged, so `git branch -d` refuses and people reach for `-D` and hope.

RepoBell already watches every repo and already knows the team's main branch. It can tell which local branches are finished and offer to delete them safely, with Undo.

## Goals

- Show, per repo, which local branches are finished.
- Delete them in a click, without ever losing work by accident.
- Offer Undo for every delete.
- Stay quiet: cleanup is never urgent.

## Non-goals

- Deleting branches on the server (GitHub, GitLab). Local only. GitHub can delete merged branches by itself with one repo setting.
- Branches that were never pushed. They are never listed.
- Old but unmerged branches ("no commits in 3 months"). Not listed.
- A cleanup screen across all repos. Each repo cleans up on its own card.
- Notifications, tray dot or tray menu lines for cleanup.

## Anti-goals

The feature fails, even if it works, if any of these happen:

- A branch with work that exists nowhere else is deleted without the user picking it on purpose.
- The branch the user is on, main, or a branch open in another worktree is deleted.
- Cleanup counts as news, nags, or makes the tray dot light up.
- Checking for old branches noticeably adds network use or slows down checks.

## What the user sees

The repo card gets one quiet line, for example "4 old branches", only when there is at least one. It sits with the other low-key card details, away from the news. It does not change the "3 new" badge, the tray dot, the tray menu or notifications.

Every listed branch was pushed before and has since been deleted on the server, so the team is done with it. Clicking the line opens a list on the card with two groups:

1. Merged: Git confirms these are fully inside the team's main. All are ticked. One button deletes the ticked ones ("Delete 3 branches").
2. May not be merged: Git can't confirm they are inside main. This usually means a squash merge, but it can also be an abandoned pull request. None are ticked; the user ticks each one on purpose. Each row shows the branch name and how long ago its last commit was, so the user can judge.

After a delete, the card shows an Undo banner in the same style as Update's Undo: "Deleted 3 branches. You can undo it." Undo brings every deleted branch back exactly as it was.

If both groups are empty, the line is not shown.

## Rules (decided)

- Only branches that were pushed at least once are listed, meaning they have an upstream set to a branch on `origin`. A branch just created from main has no commits of its own and looks "merged" to Git; without this rule RepoBell would offer to delete work the user just started.
- A branch whose upstream is the team's main (`origin/<main>`) is never listed. Git sets that upstream by itself when a branch is made with `git checkout -b new origin/main` (verified on Git 2.34.1), so such a branch may be work just started. (Claude's call, from the builder check.)
- Never listed: the checked-out branch, the local main branch, any branch checked out in a linked worktree, and any branch with no upstream.
- A branch is listed only when its upstream branch no longer exists on the server. Long-lived branches such as `develop` or `staging` stay on the server, so they are never listed, even right after they are merged into main. Teams that never delete server branches see nothing; GitHub can delete them after each merge with one repo setting. (Ramin's choice, 2026-11-10.)
- Merged means the branch tip is reachable from `origin/<main>` (the copy RepoBell already fetches). Verified on Git 2.34.1: `git branch --merged origin/main` finds normal merges and does not find squash merges.
- May not be merged means a listed branch that is not in the Merged group. Verified on Git 2.34.1: `git ls-remote --heads origin` lists remaining branches and a deleted squash-merged branch is absent.
- RepoBell never deletes or pushes anything on the server.
- Undo lasts as long as Update's Undo does: until the repo changes or the window closes. It recreates each branch at the commit it pointed to, with its upstream setting. If a branch of the same name exists again by then, that branch is skipped and the result says so.
- There is no extra prompt before deleting. The user already chose what to delete by ticking, and Undo covers mistakes.

## Server check

Today's background check downloads only main. It calls `git ls-remote` only to follow a renamed main branch. The server's branch list is therefore a new network call. To keep network use low:

- Run `git ls-remote --heads origin` only for repos that have at least one candidate branch (has an upstream on origin that is not main, not checked out here or in a worktree).
- Run it at most once an hour per repo, and also when the user opens the list. Reuse the last answer in between.
- It uses the same background rules as fetch: no hooks, no password prompts, the same SSH settings and timeouts.
- If it fails, keep showing the last good answer. With no good answer yet, show no cleanup line. A failed server check never counts as a repo error and never changes the card's "Can't check" state.

## Safety notes for the builder

- `git branch -d` refuses a branch open in a worktree. A lower-level delete such as `git update-ref -d` does not (verified on Git 2.34.1). Whatever delete command is used, skip worktree branches explicitly, using `git worktree list --porcelain`.
- Delete only if the branch still points at the commit the user saw in the list (for example `git update-ref -d refs/heads/<name> <sha>`). If it moved, skip it and say so.
- Save each branch's commit and upstream before deleting, for Undo.
- Branch names come from the repo. Pass them as separate arguments and use full ref names, never through a shell.
- Follow the existing trust rules in `docs/decisions.md`: background reads turn off hooks and fsmonitor. Delete and Undo are user actions.

## Left to the builder

- Exact card wording, layout and where the line sits on the card, within the existing card style. Text the user reads goes through the `humanizer` skill.
- How the list opens (inline expand or small panel), as long as it stays on the card.
- Which Git commands find merged branches, read upstreams and delete, within the rules above.
- How the last server answer is cached between checks.
- Tests: core rules get Rust tests on real temporary repos (merged, squash merged, never pushed, worktree, checked out, moved since listed, Undo with a name taken). The card gets UI tests like the existing ones.

## Docs to update when built

- `docs/decisions.md`: a "Branch cleanup" section with the rules above.
- `ROADMAP.md`: the new item and the current step.
- `README.md`: one line under "What It Does".
