use std::fs;

use repobell_core::fetch::fetch_main;
use repobell_core::GitError;
use tempfile::TempDir;

mod common;
use common::{commit_file, git, init_repo, push_new_commits, setup_remote_and_clone};

#[test]
fn fetch_main_updates_origin_main_to_remote_head() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_new_commits(&other, 2);

    fetch_main(&work, "main").unwrap();

    assert_eq!(
        git(&work, &["rev-parse", "origin/main"]).trim(),
        git(&other, &["rev-parse", "HEAD"]).trim()
    );
}

#[test]
fn fetch_main_keeps_head_local_main_and_working_tree() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_new_commits(&other, 2);
    let head_before = git(&work, &["rev-parse", "HEAD"]);
    let main_before = git(&work, &["rev-parse", "main"]);
    fs::write(work.join("README.md"), "unsaved edit\n").unwrap();

    fetch_main(&work, "main").unwrap();

    assert_eq!(git(&work, &["rev-parse", "HEAD"]), head_before);
    assert_eq!(git(&work, &["rev-parse", "main"]), main_before);
    assert_eq!(
        fs::read_to_string(work.join("README.md")).unwrap(),
        "unsaved edit\n"
    );
}

#[test]
fn fetch_main_missing_branch_is_failed() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());

    let result = fetch_main(&work, "nope");

    assert!(
        matches!(result, Err(GitError::Failed { .. })),
        "got {result:?}"
    );
}

#[test]
fn fetch_main_unreachable_origin_is_failed() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    git(
        &work,
        &["remote", "set-url", "origin", "/nonexistent/path.git"],
    );

    let result = fetch_main(&work, "main");

    assert!(
        matches!(result, Err(GitError::Failed { .. })),
        "got {result:?}"
    );
}

#[test]
fn fetch_main_without_remote_is_error() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("local");
    init_repo(&repo);
    commit_file(&repo, "a.txt", "a\n", "first");

    let result = fetch_main(&repo, "main");

    assert!(result.is_err(), "got {result:?}");
}

#[cfg(unix)]
#[test]
fn fetch_main_respects_repo_core_ssh_command() {
    // If the test process already has GIT_SSH_COMMAND, Git uses that instead of the
    // repo's core.sshCommand, so the result says nothing about the repo setting.
    if std::env::var_os("GIT_SSH_COMMAND").is_some() {
        return;
    }
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    init_repo(&repo);
    let marker = tmp.path().join("marker");
    let script = tmp.path().join("fake-ssh.sh");
    fs::write(
        &script,
        format!("#!/bin/sh\ntouch {}\nexit 1\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    git(
        &repo,
        &["config", "core.sshCommand", script.to_str().unwrap()],
    );
    git(
        &repo,
        &["remote", "add", "origin", "ssh://example.invalid/repo.git"],
    );

    let result = fetch_main(&repo, "main");

    assert!(result.is_err(), "got {result:?}");
    assert!(
        marker.exists(),
        "fake ssh was not run; RepoBell overrode core.sshCommand"
    );
}

#[cfg(unix)]
fn fake_script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let script = dir.join(name);
    fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[cfg(unix)]
#[test]
fn fetch_main_times_out_on_a_remote_that_never_answers() {
    use repobell_core::fetch::fetch_main_with_timeout;
    use std::time::{Duration, Instant};

    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    git(&work, &["config", "protocol.ext.allow", "always"]);
    git(&work, &["remote", "set-url", "origin", "ext::sleep 30"]);
    let start = Instant::now();

    let result = fetch_main_with_timeout(&work, "main", Duration::from_secs(1));

    assert!(
        start.elapsed() < Duration::from_secs(10),
        "did not stop in time"
    );
    let Err(err) = result else {
        panic!("expected a timeout error");
    };
    assert_eq!(
        repobell_core::classify(&err, &work),
        repobell_core::Problem::Offline
    );
    assert!(err.to_string().contains("timed out"), "{err}");
}

#[cfg(unix)]
#[test]
fn fetch_main_follows_a_renamed_default_branch() {
    let tmp = TempDir::new().unwrap();
    let (remote, work, _other) = setup_remote_and_clone(tmp.path());
    git(&remote, &["branch", "-m", "main", "trunk"]);

    fetch_main(&work, "main").unwrap();

    assert_eq!(
        repobell_core::status(&work).unwrap().main_branch.as_deref(),
        Some("trunk")
    );
    git(
        &work,
        &["rev-parse", "--verify", "refs/remotes/origin/trunk"],
    );
}

/// Answers every request with "401 Basic auth required", so Git wants a password.
#[cfg(unix)]
fn serve_401() -> u16 {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for mut conn in listener.incoming().flatten() {
            let mut buf = [0u8; 4096];
            let _ = conn.read(&mut buf);
            let _ = conn.write_all(
                b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"x\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    });
    port
}

#[cfg(unix)]
#[test]
fn fetch_main_never_runs_a_password_helper() {
    let tmp = TempDir::new().unwrap();
    let repo = tmp.path().join("repo");
    init_repo(&repo);
    let marker = tmp.path().join("asked");
    let helper = fake_script(
        tmp.path(),
        "askpass.sh",
        &format!("touch {}\necho secret", marker.display()),
    );
    git(&repo, &["config", "core.askPass", helper.to_str().unwrap()]);
    let url = format!("http://127.0.0.1:{}/repo.git", serve_401());
    git(&repo, &["remote", "add", "origin", &url]);

    // Control: plain Git, with only prompts off, does call the helper.
    let _ = common::git_expect_fail(&repo, &["fetch", "origin"]);
    assert!(marker.exists(), "control failed: helper was not called");
    fs::remove_file(&marker).unwrap();

    let result = fetch_main(&repo, "main");

    assert!(result.is_err());
    assert!(!marker.exists(), "RepoBell ran the password helper");
}

#[cfg(unix)]
fn write_script(path: &std::path::Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(unix)]
#[test]
fn fetch_does_not_run_repo_hooks() {
    let tmp = TempDir::new().unwrap();
    let (_remote, work, other) = setup_remote_and_clone(tmp.path());
    push_new_commits(&other, 1);
    let marker = tmp.path().join("hook-ran");
    // reference-transaction runs when fetch updates a ref; pre-auto-gc when gc --auto starts.
    for hook in ["reference-transaction", "pre-auto-gc"] {
        write_script(
            &work.join(".git/hooks").join(hook),
            &format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
        );
    }
    git(&work, &["config", "gc.auto", "1"]);
    git(&work, &["config", "gc.autoDetach", "false"]);

    fetch_main(&work, "main").unwrap();

    assert!(
        !marker.exists(),
        "a repo hook ran during a background fetch"
    );
    assert_eq!(
        git(&work, &["rev-parse", "origin/main"]).trim(),
        git(&other, &["rev-parse", "HEAD"]).trim()
    );
}

#[cfg(unix)]
#[test]
fn fetch_returns_when_a_leftover_helper_keeps_stderr_open() {
    if std::env::var_os("GIT_SSH_COMMAND").is_some() {
        return;
    }
    let tmp = TempDir::new().unwrap();
    let (_remote, work, _other) = setup_remote_and_clone(tmp.path());
    let script = tmp.path().join("ssh.sh");
    write_script(
        &script,
        "#!/bin/sh\nsleep 20 >/dev/null </dev/null &\necho fail >&2\nexit 1\n",
    );
    git(
        &work,
        &[
            "remote",
            "set-url",
            "origin",
            "ssh://nobody@example.invalid/x.git",
        ],
    );
    git(
        &work,
        &["config", "core.sshCommand", script.to_str().unwrap()],
    );

    let t = std::time::Instant::now();
    let result =
        repobell_core::fetch_main_with_timeout(&work, "main", std::time::Duration::from_secs(2));

    assert!(result.is_err());
    assert!(
        t.elapsed() < std::time::Duration::from_secs(8),
        "fetch took {:?}",
        t.elapsed()
    );
}
