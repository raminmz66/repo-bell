use std::process::Command;

use crate::store::Notified;
use crate::watcher::RepoCardView;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Toast {
    pub summary: String,
    pub body: String,
}

/// Escapes the three characters notification servers treat as markup.
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Sends one desktop notification. `--` keeps a summary that starts with "-" (an author name)
/// from being read as an option. notify-send 0.7.9 has no `--action`/`--wait`, so a click can't
/// be reported back; notifications therefore have no click action on this machine.
pub fn send(toast: &Toast) {
    let (summary, body) = (escape(&toast.summary), escape(&toast.body));
    // A thread reaps the child so no zombie is left behind.
    std::thread::spawn(move || {
        let _ = Command::new("notify-send")
            .args(["-a", "RepoBell", "--", &summary, &body])
            .status();
    });
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// Decides what to notify about after a check, and updates `notified` (the caller saves it when
/// the returned flag is true).
///
/// - Only teammate changes count; errored and ignored repos are skipped and keep their entry.
/// - A repo with no entry yet (first run, or newly added) is recorded silently, so history never
///   produces a notification.
/// - Muted repos are recorded but never produce a toast.
/// - One toast per check: names the repo and person for one repo, counts repos for several.
pub fn collect(repos: &[RepoCardView], notified: &mut Notified) -> (Option<Toast>, bool) {
    let mut changed = false;
    let mut news: Vec<(&RepoCardView, Vec<&crate::watcher::ChangeView>)> = Vec::new();
    for r in repos {
        if r.ignored || r.error.is_some() {
            continue;
        }
        let now: Vec<String> = r.teammate.iter().map(|c| c.sha.clone()).collect();
        match notified.get(&r.path) {
            None => {}
            Some(old) => {
                let fresh: Vec<_> = r
                    .teammate
                    .iter()
                    .filter(|c| !old.contains(&c.sha))
                    .collect();
                if !fresh.is_empty() && !r.muted {
                    news.push((r, fresh));
                }
                if *old == now {
                    continue;
                }
            }
        }
        notified.insert(r.path.clone(), now);
        changed = true;
    }
    let toast = match news.as_slice() {
        [] => None,
        [(repo, changes)] => Some(one_repo_toast(repo, changes)),
        many => {
            let names: Vec<&str> = many.iter().map(|(r, _)| r.name.as_str()).collect();
            let shown = names.iter().take(3).copied().collect::<Vec<_>>().join(", ");
            let rest = names.len().saturating_sub(3);
            Some(Toast {
                summary: format!("RepoBell: {} repos have news", names.len()),
                body: if rest > 0 {
                    format!("New changes in {shown} and {rest} more.")
                } else {
                    format!("New changes in {shown}.")
                },
            })
        }
    };
    (toast, changed)
}

fn one_repo_toast(repo: &RepoCardView, changes: &[&crate::watcher::ChangeView]) -> Toast {
    let n = changes.len();
    let mut authors: Vec<&str> = Vec::new();
    for c in changes {
        if !authors.contains(&c.author.as_str()) {
            authors.push(&c.author);
        }
    }
    let who = match authors.as_slice() {
        [one] => one.to_string(),
        [a, b] => format!("{a} and {b}"),
        [a, rest @ ..] => format!("{a} and {} others", rest.len()),
        [] => String::new(),
    };
    let latest = &changes[0].title;
    let body = if n == 1 {
        format!("{who} added 1 change to the main version: {latest}")
    } else {
        format!("{who} added {n} changes to the main version. Latest: {latest}")
    };
    Toast {
        summary: format!("{}: {n} new change{}", repo.name, plural(n)),
        body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watcher::ChangeView;
    use std::path::PathBuf;

    fn change(sha: &str, author: &str) -> ChangeView {
        ChangeView {
            author: author.into(),
            title: format!("title {sha}"),
            sha: sha.into(),
            ..Default::default()
        }
    }

    fn repo(name: &str, shas: &[(&str, &str)]) -> RepoCardView {
        RepoCardView {
            path: PathBuf::from(format!("/r/{name}")),
            name: name.into(),
            teammate: shas.iter().map(|(s, a)| change(s, a)).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn first_sight_is_silent_and_recorded() {
        let mut n = Notified::new();
        let (toast, changed) = collect(&[repo("a", &[("1", "Sara")])], &mut n);
        assert!(toast.is_none());
        assert!(changed);
        assert_eq!(n[&PathBuf::from("/r/a")], vec!["1".to_string()]);
    }

    #[test]
    fn notifies_once_per_change() {
        let mut n = Notified::new();
        collect(&[repo("a", &[("1", "Sara")])], &mut n);
        let r = repo("a", &[("2", "Sara"), ("1", "Sara")]);
        let (toast, changed) = collect(std::slice::from_ref(&r), &mut n);
        assert!(changed);
        let t = toast.unwrap();
        assert_eq!(t.summary, "a: 1 new change");
        assert_eq!(t.body, "Sara added 1 change to the main version: title 2");
        // Same state again: nothing to say, nothing to save (this is what a relaunch sees).
        let (toast, changed) = collect(&[r], &mut n);
        assert!(toast.is_none() && !changed);
    }

    #[test]
    fn muted_errored_and_ignored_stay_quiet() {
        let mut n = Notified::new();
        let base = [repo("m", &[]), repo("e", &[]), repo("i", &[])];
        collect(&base, &mut n);
        let mut m = repo("m", &[("1", "Sara")]);
        m.muted = true;
        let mut e = repo("e", &[("2", "Sara")]);
        e.error = Some("x".into());
        let mut i = repo("i", &[("3", "Sara")]);
        i.ignored = true;
        let (toast, _) = collect(&[m.clone(), e, i], &mut n);
        assert!(toast.is_none());
        // The muted repo's change is recorded, so unmuting later doesn't replay it.
        m.muted = false;
        assert!(collect(&[m], &mut n).0.is_none());
        // The errored repo keeps its entry.
        assert!(n.contains_key(&PathBuf::from("/r/e")));
    }

    #[test]
    fn several_repos_make_one_toast() {
        let mut n = Notified::new();
        let quiet = [
            repo("a", &[]),
            repo("b", &[]),
            repo("c", &[]),
            repo("d", &[]),
        ];
        collect(&quiet, &mut n);
        let busy = [
            repo("a", &[("1", "Sara")]),
            repo("b", &[("2", "Ali")]),
            repo("c", &[("3", "Ali")]),
            repo("d", &[("4", "Ali")]),
        ];
        let t = collect(&busy, &mut n).0.unwrap();
        assert_eq!(t.summary, "RepoBell: 4 repos have news");
        assert_eq!(t.body, "New changes in a, b, c and 1 more.");
    }

    #[test]
    fn people_and_markup() {
        let mut n = Notified::new();
        collect(&[repo("a", &[])], &mut n);
        let t = collect(
            &[repo("a", &[("1", "-Sara"), ("2", "Ali"), ("3", "Joe")])],
            &mut n,
        )
        .0
        .unwrap();
        assert_eq!(
            t.body,
            "-Sara and 2 others added 3 changes to the main version. Latest: title 1"
        );
        assert_eq!(escape("a<b>&c"), "a&lt;b&gt;&amp;c");
    }
}
