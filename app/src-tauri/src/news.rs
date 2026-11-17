//! The "what counts as news" rule and everything built from it (docs/decisions.md).
//! One rule feeds the tray dot, the tray headline, the tray lines and the window headline.

use crate::watcher::{CheckProgress, RepoCardView};

/// A repo is news when it can be checked and teammates added changes since the user last looked.
/// Muted repos still count (muting only silences notifications). Own changes never count.
pub fn is_news(r: &RepoCardView) -> bool {
    !r.ignored && r.error.is_none() && r.new_count > 0
}

pub fn is_problem(r: &RepoCardView) -> bool {
    !r.ignored && r.error.is_some()
}

/// Teammate changes in a feed. `listed` changes were looked at (`teammate_listed` of them from
/// teammates); the `total - listed` that were cut off count as teammates (a safe over-count).
pub fn count_new(teammate_listed: usize, listed: usize, total: usize) -> u32 {
    (teammate_listed + total.saturating_sub(listed)) as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NewsSummary {
    pub news_repos: usize,
    pub news_changes: usize,
    pub error_repos: usize,
    /// Watched folders that no longer exist. They can't be checked either.
    pub error_folders: usize,
}

pub fn summarize(repos: &[RepoCardView], missing_folders: usize) -> NewsSummary {
    let mut s = NewsSummary {
        error_folders: missing_folders,
        ..NewsSummary::default()
    };
    for r in repos {
        if is_news(r) {
            s.news_repos += 1;
            s.news_changes += r.new_count as usize;
        } else if is_problem(r) {
            s.error_repos += 1;
        }
    }
    s
}

fn plural(n: usize, one: &'static str, many: &'static str) -> &'static str {
    if n == 1 {
        one
    } else {
        many
    }
}

pub fn headline(s: &NewsSummary, offline: bool) -> String {
    if offline {
        return "Can't check your repos".to_string();
    }
    let news = (s.news_repos > 0).then(|| {
        format!(
            "{} {} {} news",
            s.news_repos,
            plural(s.news_repos, "repo", "repos"),
            plural(s.news_repos, "has", "have")
        )
    });
    let count = s.error_repos + s.error_folders;
    let errors = (count > 0).then(|| {
        // Name what can't be checked only when it is one kind of thing and no news shares the line.
        let noun = match (s.error_repos > 0, s.error_folders > 0, news.is_some()) {
            (true, false, false) => format!(" {}", plural(count, "repo", "repos")),
            (false, true, false) => format!(" {}", plural(count, "folder", "folders")),
            _ => String::new(),
        };
        format!("{count}{noun} can't be checked")
    });
    match (news, errors) {
        (Some(n), Some(e)) => format!("{n}, {e}"),
        (Some(t), None) | (None, Some(t)) => t,
        (None, None) => "All quiet".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayIcon {
    Quiet,
    News,
    CantCheck,
}

/// Everything the tray shows. The tray is rebuilt only when this changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayModel {
    pub icon: TrayIcon,
    pub headline: String,
    /// (menu id, label); the id is `repo:<path>`.
    pub lines: Vec<(String, String)>,
    pub more: Option<String>,
}

pub const TRAY_MAX_LINES: usize = 5;

/// The "Check now" tray item: its text and whether it can be clicked.
pub fn check_now_label(progress: Option<&CheckProgress>) -> (String, bool) {
    match progress {
        None => ("Check now".to_string(), true),
        Some(p) => (format!("Checking {} of {}…", p.done, p.total), false),
    }
}

pub fn tray_model(repos: &[RepoCardView], missing_folders: usize, offline: bool) -> TrayModel {
    let s = summarize(repos, missing_folders);
    let icon = if offline {
        TrayIcon::CantCheck
    } else if s.news_repos > 0 {
        TrayIcon::News
    } else {
        TrayIcon::Quiet
    };
    // News first (most changes first), then repos that can't be checked.
    let mut active: Vec<&RepoCardView> = repos
        .iter()
        .filter(|r| is_news(r) || is_problem(r))
        .collect();
    active.sort_by(|a, b| {
        (is_news(b), b.new_count, &a.name).cmp(&(is_news(a), a.new_count, &b.name))
    });
    let lines = active
        .iter()
        .take(TRAY_MAX_LINES)
        .map(|r| {
            let label = if r.error.is_some() {
                format!("{}: Can't check", r.name)
            } else {
                format!(
                    "{}: {} new {}",
                    r.name,
                    r.new_count,
                    plural(r.new_count as usize, "change", "changes")
                )
            };
            (format!("repo:{}", r.path.display()), label)
        })
        .collect();
    let more = (active.len() > TRAY_MAX_LINES)
        .then(|| format!("and {} more…", active.len() - TRAY_MAX_LINES));
    TrayModel {
        icon,
        headline: headline(&s, offline),
        lines,
        more,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo(name: &str, new_count: u32) -> RepoCardView {
        RepoCardView {
            path: PathBuf::from(format!("/r/{name}")),
            name: name.to_string(),
            new_count,
            ..Default::default()
        }
    }

    #[test]
    fn check_now_label_follows_progress() {
        assert_eq!(check_now_label(None), ("Check now".to_string(), true));
        let p = CheckProgress {
            done: 6,
            total: 15,
            current: "alpha".into(),
        };
        assert_eq!(
            check_now_label(Some(&p)),
            ("Checking 6 of 15…".to_string(), false)
        );
    }

    #[test]
    fn count_new_adds_the_cut_off_part() {
        assert_eq!(count_new(2, 5, 5), 2);
        assert_eq!(count_new(3, 200, 250), 53);
        assert_eq!(count_new(0, 0, 0), 0);
    }

    #[test]
    fn news_rule() {
        let mut muted = repo("a", 2);
        muted.muted = true;
        assert!(is_news(&muted), "muted repos still count");
        let mut errored = repo("b", 3);
        errored.error = Some("x".into());
        assert!(!is_news(&errored), "errored repos are not news");
        assert!(is_problem(&errored));
        let mut ignored = repo("c", 3);
        ignored.ignored = true;
        assert!(!is_news(&ignored));
        let mut behind_only = repo("d", 0);
        behind_only.behind = 4;
        assert!(!is_news(&behind_only), "behind is not news");
        let s = summarize(&[muted, errored, ignored, behind_only], 0);
        assert_eq!(
            s,
            NewsSummary {
                news_repos: 1,
                news_changes: 2,
                error_repos: 1,
                error_folders: 0
            }
        );
    }

    fn hf(n: usize, e: usize, folders: usize) -> String {
        headline(
            &NewsSummary {
                news_repos: n,
                news_changes: n,
                error_repos: e,
                error_folders: folders,
            },
            false,
        )
    }

    #[test]
    fn missing_folders_count_as_cant_be_checked() {
        assert_eq!(hf(0, 0, 1), "1 folder can't be checked");
        assert_eq!(hf(0, 1, 1), "2 can't be checked");
        assert_eq!(hf(0, 2, 2), "4 can't be checked");
        assert_eq!(hf(2, 0, 1), "2 repos have news, 1 can't be checked");
    }

    #[test]
    fn headlines() {
        let h = |n, e| hf(n, e, 0);
        assert_eq!(h(0, 0), "All quiet");
        assert_eq!(h(1, 0), "1 repo has news");
        assert_eq!(h(3, 0), "3 repos have news");
        assert_eq!(h(0, 2), "2 repos can't be checked");
        assert_eq!(h(0, 1), "1 repo can't be checked");
        assert_eq!(h(3, 2), "3 repos have news, 2 can't be checked");
        assert_eq!(
            headline(&NewsSummary::default(), true),
            "Can't check your repos"
        );
    }

    #[test]
    fn tray_follows_the_same_rule() {
        let mut err = repo("err", 0);
        err.error = Some("x".into());
        let mut muted = repo("muted", 1);
        muted.muted = true;
        let m = tray_model(&[repo("quiet", 0), err, muted, repo("busy", 4)], 0, false);
        assert_eq!(m.icon, TrayIcon::News);
        assert_eq!(m.headline, "2 repos have news, 1 can't be checked");
        let labels: Vec<_> = m.lines.iter().map(|l| l.1.as_str()).collect();
        assert_eq!(
            labels,
            [
                "busy: 4 new changes",
                "muted: 1 new change",
                "err: Can't check"
            ]
        );
        assert!(m.more.is_none());
        let quiet = tray_model(&[repo("quiet", 0)], 0, false);
        assert_eq!(quiet.icon, TrayIcon::Quiet);
        assert_eq!(tray_model(&[], 0, true).icon, TrayIcon::CantCheck);
    }

    #[test]
    fn tray_caps_lines() {
        let repos: Vec<_> = (0..7).map(|i| repo(&format!("r{i}"), 1)).collect();
        let m = tray_model(&repos, 0, false);
        assert_eq!(m.lines.len(), 5);
        assert_eq!(m.more.as_deref(), Some("and 2 more…"));
    }
}
