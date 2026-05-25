//! plan_ref:
//!   - 03_storage#projection-contract
//!   - 03_storage#watcher-contract

use super::WriteSuppressor;

pub(crate) struct PersistGuard {
    inner: WriteSuppressor,
}

impl PersistGuard {
    pub(crate) fn new() -> Self {
        Self {
            inner: WriteSuppressor::new(),
        }
    }

    pub(crate) fn record(&self, path: &str, content: &str) {
        if let Some((repo, repo_path)) = split(path) {
            self.inner.register_write(repo, repo_path, content);
        }
    }

    pub(crate) fn record_delete(&self, path: &str) {
        if let Some((repo, repo_path)) = split(path) {
            self.inner.register_delete(repo, repo_path);
        }
    }

    pub(crate) fn clear(&self, path: &str) {
        if let Some((repo, repo_path)) = split(path) {
            self.inner.clear(repo, repo_path);
        }
    }

    pub(crate) fn should_ignore(&self, repo_root: &std::path::Path, path: &str) -> bool {
        split(path)
            .map(|(repo, repo_path)| self.inner.should_suppress(repo, repo_root, repo_path))
            .unwrap_or(false)
    }
}

fn split(path: &str) -> Option<(&str, &str)> {
    let normalized = path.trim_matches('/');
    let (repo, repo_path) = normalized.split_once('/')?;
    Some((repo, repo_path))
}

#[cfg(test)]
mod tests {
    use super::PersistGuard;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[test]
    fn poisoned_lock_stops_ignoring_without_panicking() {
        let guard = PersistGuard::new();
        let dir = tempdir().expect("tempdir");
        assert!(!guard.should_ignore(dir.path(), "notes/a.md"));
        guard.record("notes/a.md", "content");
        guard.record_delete("notes/a.md");
        guard.clear("notes/a.md");
    }

    #[cfg(unix)]
    #[test]
    fn delete_guard_fails_closed_when_target_is_unstatable() {
        let guard = PersistGuard::new();
        let dir = tempdir().expect("tempdir");
        let notes = dir.path().join("default").join("notes");
        std::fs::create_dir_all(&notes).expect("mkdir");
        std::fs::write(notes.join("a.md"), "content").expect("write");
        let original = std::fs::metadata(&notes).expect("metadata").permissions();
        let mut blocked = original.clone();
        blocked.set_mode(0o000);
        guard.record_delete("default/notes/a.md");
        std::fs::set_permissions(&notes, blocked).expect("chmod 000");

        let ignored = guard.should_ignore(dir.path(), "default/notes/a.md");

        std::fs::set_permissions(&notes, original).expect("restore perms");
        assert!(!ignored);
    }
}
