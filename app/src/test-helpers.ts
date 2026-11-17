import type { AppStateView, RepoCardView, FeedSummary, UpdateView } from "./lib/types";

export function makeRepo(overrides: Partial<RepoCardView> = {}): RepoCardView {
  return {
    path: "/home/user/projects/my-repo",
    name: "my-repo",
    folder: "/home/user/projects",
    main_branch: "main",
    branch: "main",
    on_main: true,
    error: null,
    error_kind: null,
    behind: 0,
    ahead: 0,
    unsaved: false,
    main_behind: 0,
    muted: false,
    ignored: false,
    new_count: 0,
    feed: null,
    can_undo: false,
    old_branches: [],
    can_undo_cleanup: false,
    ...overrides,
  };
}

export function makeFeed(overrides: Partial<FeedSummary> = {}): FeedSummary {
  const t = Math.floor(Date.now() / 1000) - 300;
  return {
    total_changes: 2,
    newest_time: t,
    changes: [
      { author: "Alex", title: "Fix login timeout", sha: "abc1234567890", timestamp: t, mine: false },
      { author: "Alex", title: "Update header spacing", sha: "def1234567890", timestamp: t - 10, mine: false },
    ],
    files_changed: ["src/api.ts", "README.md"],
    ...overrides,
  };
}

export function makeUpdate(overrides: Partial<UpdateView> = {}): UpdateView {
  return {
    current: "0.1.0",
    can_update: true,
    deb: true,
    status: "idle",
    checked_at: null,
    available: null,
    downloaded: 0,
    total: null,
    error: null,
    busy: false,
    ...overrides,
  };
}

export function makeState(overrides: Partial<AppStateView> = {}): AppStateView {
  return {
    first_run: false,
    offline: false,
    last_checked: Math.floor(Date.now() / 1000) - 120,
    checking: false,
    headline: "All quiet",
    news_repos: 0,
    news_changes: 0,
    error_repos: 0,
    folder_errors: [],
    progress: null,
    settings: {
      folders: ["/home/user/projects"],
      ignored: [],
      muted: [],
      editor: "VS Code",
      check_minutes: 5,
      start_at_login: true,
    },
    repos: [makeRepo({ path: "/home/user/projects/repo-one", name: "repo-one" })],
    update: makeUpdate(),
    ...overrides,
  };
}
