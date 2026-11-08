use std::collections::HashSet;
use std::process::Command;

use crate::watcher::RepoCardView;

pub fn send_notification(summary: &str, body: &str) {
    let _ = Command::new("notify-send")
        .args(["-a", "RepoBell", summary, body])
        .spawn();
}

pub fn notify_news(repos: &[RepoCardView], previously_seen_shas: &HashSet<String>) {
    // Collect unmuted repos that have new incoming teammate commits not yet notified
    let mut repos_with_new_teammate_commits = Vec::new();

    for r in repos {
        if r.ignored || r.muted {
            continue;
        }
        if let Some(ref feed) = r.feed {
            // Find incoming commits from teammates (mine == false) that weren't in previously_seen_shas
            let new_teammate_changes: Vec<_> = feed
                .changes
                .iter()
                .filter(|c| !c.mine && !previously_seen_shas.contains(&c.sha))
                .collect();

            if !new_teammate_changes.is_empty() {
                repos_with_new_teammate_commits.push((r, new_teammate_changes));
            }
        }
    }

    if repos_with_new_teammate_commits.is_empty() {
        return;
    }

    if repos_with_new_teammate_commits.len() == 1 {
        let (repo, changes) = &repos_with_new_teammate_commits[0];
        let count = changes.len();
        let author = &changes[0].author;
        let title = &changes[0].title;
        let summary = format!("{}: {} new change{}", repo.name, count, if count == 1 { "" } else { "s" });
        let body = if count == 1 {
            format!("{} added 1 change to the main version: {}", author, title)
        } else {
            format!("{} added {} changes to the main version. {}, and {} more.", author, count, title, count - 1)
        };
        send_notification(&summary, &body);
    } else {
        let count = repos_with_new_teammate_commits.len();
        let names: Vec<_> = repos_with_new_teammate_commits
            .iter()
            .take(3)
            .map(|(r, _)| r.name.as_str())
            .collect();
        let summary = format!("RepoBell: {} repos have news", count);
        let body = format!("New changes were pushed to {}.", names.join(", "));
        send_notification(&summary, &body);
    }
}
