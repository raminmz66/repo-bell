use std::path::PathBuf;
use repobell_lib::store::Settings;
use repobell_lib::watcher::{apply_settings_to_repos, RepoCardView};

#[test]
fn test_apply_settings_to_repos_updates_ignored_and_muted() {
    let repo1_path = PathBuf::from("/var/www/html/repo1");
    let repo2_path = PathBuf::from("/var/www/html/repo2");

    let mut repos = vec![
        RepoCardView {
            path: repo1_path.clone(),
            name: "repo1".to_string(),
            folder: PathBuf::from("/var/www/html"),
            main_branch: "main".to_string(),
            error: None,
            behind: 2,
            ahead: 0,
            unsaved: false,
            muted: false,
            ignored: false,
            feed: None,
            undo_head: None,
        },
        RepoCardView {
            path: repo2_path.clone(),
            name: "repo2".to_string(),
            folder: PathBuf::from("/var/www/html"),
            main_branch: "main".to_string(),
            error: None,
            behind: 0,
            ahead: 0,
            unsaved: false,
            muted: false,
            ignored: false,
            feed: None,
            undo_head: None,
        },
    ];

    let mut settings = Settings::default();
    settings.folders = vec![PathBuf::from("/var/www/html")];
    // User turned off repo1 and muted repo2
    settings.ignored = vec![repo1_path.clone()];
    settings.muted = vec![repo2_path.clone()];

    apply_settings_to_repos(&mut repos, &settings);

    assert!(repos[0].ignored, "repo1 should be ignored according to latest settings");
    assert!(!repos[0].muted, "repo1 should not be muted");

    assert!(!repos[1].ignored, "repo2 should not be ignored");
    assert!(repos[1].muted, "repo2 should be muted according to latest settings");
}
