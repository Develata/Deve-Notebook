//! plan_ref:
//!   - 03_storage#repo-runtime-layout

use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

pub const NOTE_GIT_DIR: &str = ".notegit";
pub const GIT_DIR: &str = ".git";
pub const NOTE_GIT_IGNORE_PATTERN: &str = ".notegit/";

pub fn repo_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(NOTE_GIT_DIR)
}

pub fn is_internal_repo_path(path: &str) -> bool {
    crate::utils::path::to_forward_slash(path)
        .split('/')
        .any(is_internal_repo_segment)
}

pub fn is_internal_repo_segment(segment: &str) -> bool {
    matches!(segment, NOTE_GIT_DIR | GIT_DIR)
}

pub fn repo_keys_dir(repo_root: &Path) -> PathBuf {
    repo_dir(repo_root).join("keys")
}

pub fn host_dir(ledger_root: &Path) -> PathBuf {
    ledger_root.join(".host")
}

pub fn host_keys_dir(ledger_root: &Path) -> PathBuf {
    host_dir(ledger_root).join("keys")
}

pub fn gitignore_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".gitignore")
}

pub fn gitignore_ignores_notegit(repo_root: &Path) -> std::io::Result<bool> {
    let path = gitignore_path(repo_root);
    let Some(content) = read_regular_gitignore(&path)? else {
        return Ok(false);
    };
    Ok(gitignore_effectively_ignores_notegit(&content))
}

pub fn ensure_gitignore_ignores_notegit(repo_root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(repo_root)?;
    let path = gitignore_path(repo_root);
    let existing = read_regular_gitignore(&path)?.unwrap_or_default();
    if gitignore_effectively_ignores_notegit(&existing) {
        return Ok(());
    }

    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(NOTE_GIT_IGNORE_PATTERN);
    next.push('\n');
    std::fs::write(path, next)
}

fn read_regular_gitignore(path: &Path) -> std::io::Result<Option<String>> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err),
    };
    if meta.file_type().is_symlink() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("refusing to follow symlinked .gitignore {}", path.display()),
        ));
    }
    if !meta.is_file() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(".gitignore is not a regular file {}", path.display()),
        ));
    }
    std::fs::read_to_string(path).map(Some)
}

fn gitignore_effectively_ignores_notegit(content: &str) -> bool {
    let mut protected = false;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (negated, pattern) = match line.strip_prefix('!') {
            Some(pattern) => (true, pattern.trim()),
            None => (false, line),
        };
        if gitignore_pattern_mentions_notegit(pattern) {
            protected = !negated;
        }
    }
    protected
}

fn gitignore_pattern_mentions_notegit(pattern: &str) -> bool {
    let pattern = pattern.trim_start_matches('/');
    pattern == ".notegit" || pattern.starts_with(".notegit/")
}

#[cfg(test)]
mod tests {
    use super::{
        ensure_gitignore_ignores_notegit, gitignore_ignores_notegit, is_internal_repo_path,
    };

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[test]
    fn internal_repo_path_uses_segment_semantics() {
        assert!(is_internal_repo_path(".notegit/state.json"));
        assert!(is_internal_repo_path("notes/.notegit/state.json"));
        assert!(is_internal_repo_path(".git/config"));
        assert!(is_internal_repo_path("notes/.git/objects/x"));
        assert!(!is_internal_repo_path(".gitignore"));
        assert!(!is_internal_repo_path(".notegit-backup/state.json"));
        assert!(!is_internal_repo_path(".git-backup/config"));
    }

    #[test]
    fn ensure_gitignore_ignores_notegit_is_idempotent() {
        let dir = tempfile::tempdir().expect("tempdir");

        ensure_gitignore_ignores_notegit(dir.path()).expect("ensure");
        ensure_gitignore_ignores_notegit(dir.path()).expect("ensure again");

        let content = std::fs::read_to_string(dir.path().join(".gitignore")).expect("read");
        assert_eq!(content.matches(".notegit/").count(), 1);
        assert!(gitignore_ignores_notegit(dir.path()).expect("status"));
    }

    #[test]
    fn existing_root_notegit_gitignore_pattern_counts_as_protected() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".gitignore"), "/.notegit/\n").expect("write");

        ensure_gitignore_ignores_notegit(dir.path()).expect("ensure");

        let content = std::fs::read_to_string(dir.path().join(".gitignore")).expect("read");
        assert_eq!(content, "/.notegit/\n");
        assert!(gitignore_ignores_notegit(dir.path()).expect("status"));
    }

    #[test]
    fn later_gitignore_negation_is_not_reported_as_protected() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".gitignore"), ".notegit/\n!.notegit/\n").expect("write");

        assert!(!gitignore_ignores_notegit(dir.path()).expect("status"));

        ensure_gitignore_ignores_notegit(dir.path()).expect("ensure");
        let content = std::fs::read_to_string(dir.path().join(".gitignore")).expect("read");
        assert_eq!(content.lines().last(), Some(".notegit/"));
        assert!(gitignore_ignores_notegit(dir.path()).expect("status"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_gitignore_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = dir.path().join("outside");
        std::fs::write(&outside, "# outside\n").expect("outside");
        symlink(&outside, dir.path().join(".gitignore")).expect("symlink");

        let err = ensure_gitignore_ignores_notegit(dir.path())
            .expect_err("symlinked .gitignore must fail closed");

        assert!(
            err.to_string()
                .contains("refusing to follow symlinked .gitignore")
        );
    }
}
