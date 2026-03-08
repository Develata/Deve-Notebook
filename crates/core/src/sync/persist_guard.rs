use crate::source_control::pending_fs;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::debug;

const PERSISTED_WRITE_TTL: Duration = Duration::from_secs(2);
const PERSISTED_WRITE_MATCH_BUDGET: u8 = 4;

enum PersistedAction {
    Write { content_hash: String },
}

struct PersistedWrite {
    action: PersistedAction,
    recorded_at: Instant,
    remaining_hits: u8,
}

pub(super) struct PersistGuard {
    recent_writes: Mutex<HashMap<String, PersistedWrite>>,
}

impl PersistGuard {
    pub(super) fn new() -> Self {
        Self {
            recent_writes: Mutex::new(HashMap::new()),
        }
    }

    pub(super) fn record(&self, path: &str, content: &str) {
        self.insert(
            path,
            PersistedAction::Write {
                content_hash: pending_fs::content_hash(content),
            },
        );
    }

    pub(super) fn clear(&self, path: &str) {
        let mut guard = self.recent_writes.lock().unwrap();
        guard.remove(path);
    }

    pub(super) fn should_ignore(&self, vault_root: &Path, path: &str) -> bool {
        let mut guard = self.recent_writes.lock().unwrap();
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
        let mut guard = self.recent_writes.lock().unwrap();
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
}
