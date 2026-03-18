use crate::source_control::pending_fs;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tracing::{debug, warn};

const PERSISTED_WRITE_TTL: Duration = Duration::from_secs(2);
const PERSISTED_WRITE_MATCH_BUDGET: u8 = 4;

enum PersistedAction {
    Write { content_hash: String },
    Delete,
}

struct PersistedWrite {
    action: PersistedAction,
    recorded_at: Instant,
    remaining_hits: u8,
}

pub(crate) struct PersistGuard {
    recent_writes: Mutex<HashMap<String, PersistedWrite>>,
}

impl PersistGuard {
    pub(crate) fn new() -> Self {
        Self {
            recent_writes: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn record(&self, path: &str, content: &str) {
        self.insert(
            path,
            PersistedAction::Write {
                content_hash: pending_fs::content_hash(content),
            },
        );
    }

    pub(crate) fn record_delete(&self, path: &str) {
        self.insert(path, PersistedAction::Delete);
    }

    pub(crate) fn clear(&self, path: &str) {
        let Some(mut guard) = self.lock_recent_writes() else {
            return;
        };
        guard.remove(path);
    }

    pub(crate) fn should_ignore(&self, vault_root: &Path, path: &str) -> bool {
        let Some(mut guard) = self.lock_recent_writes() else {
            return false;
        };
        let Some(entry) = guard.get_mut(path) else {
            return false;
        };
        if entry.recorded_at.elapsed() > PERSISTED_WRITE_TTL {
            guard.remove(path);
            return false;
        }

        let matched = match &entry.action {
            PersistedAction::Write { content_hash } => {
                let Ok(content) = std::fs::read_to_string(vault_root.join(path)) else {
                    guard.remove(path);
                    return false;
                };
                if pending_fs::content_hash(&content) != *content_hash {
                    guard.remove(path);
                    return false;
                }
                true
            }
            PersistedAction::Delete => {
                if vault_root.join(path).exists() {
                    guard.remove(path);
                    return false;
                }
                true
            }
        };

        if entry.remaining_hits > 1 {
            entry.remaining_hits -= 1;
        } else {
            guard.remove(path);
        }
        if matched {
            debug!(
                "SyncManager: ignored self-persisted watcher event for {}",
                path
            );
        }
        matched
    }

    fn insert(&self, path: &str, action: PersistedAction) {
        let Some(mut guard) = self.lock_recent_writes() else {
            return;
        };
        guard.retain(|_, entry| entry.recorded_at.elapsed() <= PERSISTED_WRITE_TTL);
        guard.insert(
            path.to_string(),
            PersistedWrite {
                action,
                recorded_at: Instant::now(),
                remaining_hits: PERSISTED_WRITE_MATCH_BUDGET,
            },
        );
    }

    fn lock_recent_writes(&self) -> Option<MutexGuard<'_, HashMap<String, PersistedWrite>>> {
        match self.recent_writes.lock() {
            Ok(guard) => Some(guard),
            Err(_) => {
                warn!("PersistGuard: recent_writes 锁已损坏，按 fail-closed 处理");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PersistGuard;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use tempfile::tempdir;

    #[test]
    fn poisoned_lock_stops_ignoring_without_panicking() {
        let guard = PersistGuard::new();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _held = guard.recent_writes.lock().expect("lock persist guard");
            panic!("poison persist guard");
        }));

        let dir = tempdir().expect("tempdir");
        assert!(!guard.should_ignore(dir.path(), "notes/a.md"));
        guard.record("notes/a.md", "content");
        guard.record_delete("notes/a.md");
        guard.clear("notes/a.md");
    }
}
