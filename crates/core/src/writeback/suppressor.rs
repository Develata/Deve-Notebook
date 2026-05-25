//! plan_ref:
//!   - 03_storage#watcher-contract

use crate::source_control::pending_fs;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

const DEFAULT_WINDOW: Duration = Duration::from_secs(2);
const DEFAULT_HITS: u8 = 4;

enum Fingerprint {
    Write(String),
    Delete,
}

struct Entry {
    fingerprint: Fingerprint,
    at: Instant,
    hits: u8,
}

pub(crate) struct WriteSuppressor {
    repos: Mutex<HashMap<String, HashMap<String, Entry>>>,
    window: Duration,
}

impl WriteSuppressor {
    pub(crate) fn new() -> Self {
        Self {
            repos: Mutex::new(HashMap::new()),
            window: DEFAULT_WINDOW,
        }
    }

    pub(crate) fn register_write(&self, repo: &str, path: &str, content: &str) {
        self.insert(
            repo,
            path,
            Fingerprint::Write(pending_fs::content_hash(content)),
        );
    }

    pub(crate) fn register_delete(&self, repo: &str, path: &str) {
        self.insert(repo, path, Fingerprint::Delete);
    }

    pub(crate) fn clear(&self, repo: &str, path: &str) {
        if let Some(mut guard) = self.lock()
            && let Some(bucket) = guard.get_mut(repo)
        {
            bucket.remove(path);
        }
    }

    pub(crate) fn should_suppress(&self, repo: &str, root: &Path, path: &str) -> bool {
        let Some(mut guard) = self.lock() else {
            return false;
        };
        let Some(bucket) = guard.get_mut(repo) else {
            return false;
        };
        let Some(entry) = bucket.get_mut(path) else {
            return false;
        };
        if entry.at.elapsed() > self.window {
            bucket.remove(path);
            return false;
        }
        let full = root.join(path);
        let matched = match &entry.fingerprint {
            Fingerprint::Write(hash) => std::fs::read_to_string(&full)
                .map(|content| pending_fs::content_hash(&content) == *hash)
                .unwrap_or(false),
            Fingerprint::Delete => full.try_exists().map(|exists| !exists).unwrap_or(false),
        };
        if matched && entry.hits > 1 {
            entry.hits -= 1;
        } else {
            bucket.remove(path);
        }
        matched
    }

    fn insert(&self, repo: &str, path: &str, fingerprint: Fingerprint) {
        let Some(mut guard) = self.lock() else {
            return;
        };
        let bucket = guard.entry(repo.to_string()).or_default();
        bucket.retain(|_, entry| entry.at.elapsed() <= self.window);
        bucket.insert(
            path.to_string(),
            Entry {
                fingerprint,
                at: Instant::now(),
                hits: DEFAULT_HITS,
            },
        );
    }

    fn lock(&self) -> Option<MutexGuard<'_, HashMap<String, HashMap<String, Entry>>>> {
        self.repos.lock().ok()
    }
}
