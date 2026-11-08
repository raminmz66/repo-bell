export interface Settings {
  folders: string[];
  ignored: string[];
  muted: string[];
  editor: string | null;
  check_minutes: number;
  start_at_login: boolean;
}

export interface ChangeView {
  author: string;
  title: string;
  sha: string;
  timestamp: number;
  mine: boolean;
}

export interface AuthorDayGroup {
  author: string;
  mine: boolean;
  changes: ChangeView[];
}

export interface DayGroup {
  dayKey: string;
  dayLabel: string;
  authors: AuthorDayGroup[];
}

export interface FeedSummary {
  total_changes: number;
  author_summary: string;
  newest_time: number | null;
  changes: ChangeView[];
  files_changed: string[];
}

export interface RepoCardView {
  path: string;
  name: string;
  folder: string;
  main_branch: string;
  error: string | null;
  behind: number;
  ahead: number;
  unsaved: boolean;
  muted: boolean;
  ignored: boolean;
  feed: FeedSummary | null;
  undo_head: string | null;
}

export interface AppStateView {
  repos: RepoCardView[];
  settings: Settings;
  last_checked: number | null;
  offline: boolean;
  first_run: boolean;
}

export interface UpdateOutcomeView {
  success: boolean;
  downloads: number;
  undo_head: string | null;
  conflict_files: string[];
  message: string;
}

export interface SuggestedFolder {
  path: string;
  display: string;
  repos: string[];
}
