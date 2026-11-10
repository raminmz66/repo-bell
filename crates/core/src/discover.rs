use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Folder names never searched: dependency, build and cache folders.
pub const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "vendor",
    "dist",
    "build",
    "venv",
    "__pycache__",
];

/// Finds Git repos inside `roots`, up to `max_depth` folders deep (a root itself is depth 0).
///
/// Rules:
/// - A repo is a folder that contains `.git` (a folder, or a file as in submodules).
/// - Linked worktrees (`git worktree add`) are skipped: the main checkout already covers them.
/// - Once a folder has `.git`, nothing inside it is searched (no nested repos or submodules).
/// - Hidden folders (name starts with `.`) and `SKIP_DIRS` are never searched.
/// - Missing or unreadable folders are skipped silently.
/// - Symlinks are not followed.
/// - The result is sorted and has no duplicates, even when roots overlap.
pub fn discover(roots: &[PathBuf], max_depth: usize) -> Vec<PathBuf> {
    let mut found = BTreeSet::new();
    let mut stack: Vec<(PathBuf, usize)> = roots.iter().map(|r| (r.clone(), 0)).collect();
    while let Some((dir, depth)) = stack.pop() {
        if dir.join(".git").exists() {
            if !is_linked_worktree(&dir) {
                found.insert(dir);
            }
            continue;
        }
        if depth >= max_depth {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') || SKIP_DIRS.contains(&name.as_ref()) {
                continue;
            }
            if entry.file_type().is_ok_and(|t| t.is_dir()) {
                stack.push((entry.path(), depth + 1));
            }
        }
    }
    found.into_iter().collect()
}

/// A linked worktree's `.git` is a file pointing at a gitdir that holds a `commondir` file.
/// A submodule's `.git` file points at a gitdir without one.
fn is_linked_worktree(dir: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(dir.join(".git")) else {
        return false; // `.git` is a folder: a normal repo
    };
    let Some(gitdir) = text.trim().strip_prefix("gitdir:") else {
        return false;
    };
    dir.join(gitdir.trim()).join("commondir").is_file()
}
