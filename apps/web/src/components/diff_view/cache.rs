use super::model::DiffAlgorithm;
use super::model::LineView;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct DiffCacheEntry {
    pub key: String,
    pub value: DiffComputeValue,
}

pub type DiffLines = (Vec<LineView>, Vec<LineView>);
pub type DiffComputeValue = (DiffLines, DiffAlgorithm);

struct DiffCache {
    max_entries: usize,
    entries: VecDeque<DiffCacheEntry>,
}

impl DiffCache {
    fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: VecDeque::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<DiffComputeValue> {
        let idx = self.entries.iter().position(|e| e.key == key)?;
        let entry = self.entries.remove(idx)?;
        let value = entry.value.clone();
        self.entries.push_front(entry);
        Some(value)
    }

    fn put(&mut self, key: String, value: DiffComputeValue) {
        if let Some(idx) = self.entries.iter().position(|e| e.key == key) {
            let _ = self.entries.remove(idx);
        }
        self.entries.push_front(DiffCacheEntry { key, value });
        while self.entries.len() > self.max_entries {
            let _ = self.entries.pop_back();
        }
    }
}

thread_local! {
    static DIFF_CACHE: RefCell<DiffCache> = RefCell::new(DiffCache::new(50));
}

pub fn build_key(
    repo: &str,
    path: &str,
    old_content: &str,
    new_content: &str,
    mode: &str,
    context_lines: usize,
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    old_content.hash(&mut hasher);
    new_content.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{}|{}|{}|{}|{:016x}", repo, path, mode, context_lines, hash)
}

pub fn cache_get(key: &str) -> Option<DiffComputeValue> {
    DIFF_CACHE.with(|c| c.borrow_mut().get(key))
}

pub fn cache_put(key: String, value: DiffComputeValue) {
    DIFF_CACHE.with(|c| c.borrow_mut().put(key, value));
}
