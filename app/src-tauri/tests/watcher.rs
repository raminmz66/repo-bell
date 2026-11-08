#[test]
fn test_author_summary_formatting() {
    let single_change = "Sara added 1 change to the main version";
    let multi_change = "Sara added 4 changes to the main version";
    assert_eq!(single_change, format!("{} added {} change to the main version", "Sara", 1));
    assert_eq!(multi_change, format!("{} added {} changes to the main version", "Sara", 4));
}

#[test]
fn test_headline_formatting() {
    let quiet = "All quiet";
    let one_news = "1 repo has news";
    let multi_news = "3 repos have news";
    let offline = "Can't check your repos";

    assert_eq!(quiet, "All quiet");
    assert_eq!(one_news, format!("{} repo has news", 1));
    assert_eq!(multi_news, format!("{} repos have news", 3));
    assert_eq!(offline, "Can't check your repos");
}
