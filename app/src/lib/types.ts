// Shapes of the Tauri backend's IPC contract (app/src-tauri/src/actions.rs and watcher.rs).

export interface Settings {
  folders: string[];
  ignored: string[];
  muted: string[];
  editor: string | null;
  check_minutes: number;
  start_at_login: boolean;
  setup_done?: boolean;
}

export interface ChangeView {
  author: string;
  title: string;
  sha: string; // never shown: people who don't know Git can't use it
  timestamp: number;
  mine: boolean;
}

export interface FeedSummary {
  total_changes: number;
  newest_time: number | null;
  changes: ChangeView[];
  files_changed: string[];
}

export type ErrorKind =
  | "offline"
  | "login"
  | "not_found"
  | "folder_missing"
  | "not_a_repo"
  | "other";

export interface OldBranch {
  name: string;
  sha: string;
  merged: boolean;
  last_commit_time: number; // Unix seconds
}

export interface RepoCardView {
  path: string;
  name: string;
  folder: string;
  main_branch: string;
  branch: string | null;
  on_main: boolean;
  error: string | null;
  error_kind: ErrorKind | null;
  behind: number;
  ahead: number;
  unsaved: boolean;
  /** Side branch only: team commits the local main branch lacks. 0 on main. */
  main_behind: number;
  muted: boolean;
  ignored: boolean;
  new_count: number;
  feed: FeedSummary | null;
  can_undo: boolean;
  old_branches: OldBranch[];
  can_undo_cleanup: boolean;
}

export interface FolderError {
  path: string;
  message: string;
}

export interface AppStateView {
  repos: RepoCardView[];
  settings: Settings;
  last_checked: number | null;
  offline: boolean;
  first_run: boolean;
  checking: boolean;
  headline: string;
  news_repos: number;
  news_changes: number;
  error_repos: number;
  folder_errors: FolderError[];
}

export interface PlanView {
  kind:
    | "up_to_date"
    | "fast_forward"
    | "combine"
    | "detached"
    | "no_upstream"
    | "in_progress"
    | "side_branch";
  downloads: number;
  yours: number;
  set_aside: boolean;
  message: string;
}

export interface UpdateResult {
  status: "updated" | "up_to_date" | "conflict" | "plan_changed" | "blocked" | "edits_in_stash";
  message: string;
  downloads: number;
  conflict_files: string[];
  plan: PlanView | null;
  stash: string | null;
  stash_command: string | null;
}

export interface SimpleResult {
  status: string;
  message: string;
  detail: string | null;
}

export interface SuggestedFolder {
  path: string;
  display: string;
  /** Full paths of the repos found. */
  repos: string[];
}
