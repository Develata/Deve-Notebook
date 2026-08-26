//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use crate::source_control::pending_fs;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

const DEFAULT_WINDOW: Duration = Duration::from_secs(2);
const DEFAULT_HITS: u8 = 4;
const GLOBAL_GC_INTERVAL: u64 = 64;

#[derive(Clone)]
enum Fingerprint {
    Write(String),
    Delete,
}

struct Entry {
    fingerprint: Fingerprint,
    at: Instant,
    hits: u8,
    generation: u64,
}

#[derive(Default)]
struct SuppressorState {
    next_generation: u64,
    repos: HashMap<String, HashMap<String, Entry>>,
}

struct Claim {
    fingerprint: Fingerprint,
    generation: u64,
}

pub(crate) struct WriteSuppressor {
    state: Mutex<SuppressorState>,
    window: Duration,
}

impl WriteSuppressor {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(SuppressorState::default()),
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
            && let Some(bucket) = guard.repos.get_mut(repo)
        {
            bucket.remove(path);
            if bucket.is_empty() {
                guard.repos.remove(repo);
            }
        }
    }

    pub(crate) fn should_suppress(&self, repo: &str, root: &Path, path: &str) -> bool {
        let Some(claim) = self.claim(repo, path) else {
            return false;
        };
        let full = root.join(path);
        let matched = match &claim.fingerprint {
            Fingerprint::Write(hash) => std::fs::read_to_string(&full)
                .map(|content| pending_fs::content_hash(&content) == *hash)
                .unwrap_or(false),
            Fingerprint::Delete => full.try_exists().map(|exists| !exists).unwrap_or(false),
        };
        self.settle(repo, path, claim.generation, matched)
    }

    fn insert(&self, repo: &str, path: &str, fingerprint: Fingerprint) {
        let Some(mut guard) = self.lock() else {
            return;
        };
        let now = Instant::now();
        guard.next_generation = match guard.next_generation.checked_add(1) {
            Some(generation) => generation,
            None => {
                // Fail closed: retiring suppression state can only surface later events as external.
                guard.repos.clear();
                1
            }
        };
        let generation = guard.next_generation;
        if generation % GLOBAL_GC_INTERVAL == 0 {
            guard.repos.retain(|_, bucket| {
                bucket.retain(|_, entry| now.duration_since(entry.at) <= self.window);
                !bucket.is_empty()
            });
        } else {
            let remove_bucket = guard.repos.get_mut(repo).is_some_and(|bucket| {
                bucket.retain(|_, entry| now.duration_since(entry.at) <= self.window);
                bucket.is_empty()
            });
            if remove_bucket {
                guard.repos.remove(repo);
            }
        }
        let bucket = guard.repos.entry(repo.to_string()).or_default();
        bucket.insert(
            path.to_string(),
            Entry {
                fingerprint,
                at: now,
                hits: DEFAULT_HITS,
                generation,
            },
        );
    }

    fn claim(&self, repo: &str, path: &str) -> Option<Claim> {
        let mut guard = self.lock()?;
        let bucket = guard.repos.get_mut(repo)?;
        let expired = bucket
            .get(path)
            .is_some_and(|entry| entry.at.elapsed() > self.window);
        if expired {
            bucket.remove(path);
            if bucket.is_empty() {
                guard.repos.remove(repo);
            }
            return None;
        }
        let entry = bucket.get(path)?;
        Some(Claim {
            fingerprint: entry.fingerprint.clone(),
            generation: entry.generation,
        })
    }

    fn settle(&self, repo: &str, path: &str, generation: u64, matched: bool) -> bool {
        let Some(mut guard) = self.lock() else {
            return false;
        };
        let Some(bucket) = guard.repos.get_mut(repo) else {
            return false;
        };
        let current_matches = bucket
            .get(path)
            .is_some_and(|entry| entry.generation == generation);
        if current_matches {
            let keep = matched && bucket.get(path).is_some_and(|entry| entry.hits > 1);
            if keep {
                if let Some(entry) = bucket.get_mut(path) {
                    entry.hits -= 1;
                }
            } else {
                bucket.remove(path);
            }
        }
        if bucket.is_empty() {
            guard.repos.remove(repo);
        }
        matched
    }

    fn lock(&self) -> Option<MutexGuard<'_, SuppressorState>> {
        self.state.lock().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_suppressor_stale_claim_does_not_consume_new_registration() -> anyhow::Result<()> {
        let root = tempfile::tempdir()?;
        let suppressor = WriteSuppressor::new();
        suppressor.register_write("repo", "note.md", "old");
        let old_claim = suppressor.claim("repo", "note.md").expect("old claim");

        suppressor.register_write("repo", "note.md", "new");
        assert!(suppressor.settle("repo", "note.md", old_claim.generation, true));
        std::fs::write(root.path().join("note.md"), "new")?;

        assert!(suppressor.should_suppress("repo", root.path(), "note.md"));
        let guard = suppressor.lock().expect("state lock");
        assert_eq!(guard.repos["repo"]["note.md"].hits, DEFAULT_HITS - 1);
        Ok(())
    }

    #[test]
    fn write_suppressor_retires_empty_repo_buckets() {
        let suppressor = WriteSuppressor::new();
        suppressor.register_delete("repo", "gone.md");
        suppressor.clear("repo", "gone.md");
        assert!(suppressor.lock().expect("state lock").repos.is_empty());

        suppressor.register_delete("repo", "expired.md");
        {
            let mut guard = suppressor.lock().expect("state lock");
            guard
                .repos
                .get_mut("repo")
                .expect("repo bucket")
                .get_mut("expired.md")
                .expect("suppression entry")
                .at = Instant::now()
                .checked_sub(DEFAULT_WINDOW + Duration::from_millis(1))
                .expect("representable expired instant");
        }
        assert!(!suppressor.should_suppress("repo", Path::new("."), "expired.md"));
        assert!(suppressor.lock().expect("state lock").repos.is_empty());
    }
}
