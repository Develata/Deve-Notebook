//! Scope-local immutable backend projection LRU.
//! plan_ref:
//!   - 10_rendering#large-document-runtime

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, OnceLock};

use deve_core::models::PeerId;
use deve_core::source_control::CommitFileDiffTarget;
use deve_core::source_control::diff_projection::{
    DiffProjection, MAX_DIFF_PROJECTION_BYTES, projection_wire_size,
};

const MAX_PROJECTIONS: usize = 4;

struct CacheEntry {
    key: String,
    bytes: usize,
    projection: Arc<DiffProjection>,
}

#[derive(Default)]
struct ProjectionCache {
    scope: Option<String>,
    bytes: usize,
    entries: VecDeque<CacheEntry>,
}

fn cache() -> &'static Mutex<ProjectionCache> {
    static CACHE: OnceLock<Mutex<ProjectionCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(ProjectionCache::default()))
}

pub fn projection_scope_key(
    repo_id: Option<&str>,
    branch: Option<&PeerId>,
    scope_nonce: u64,
) -> String {
    format!(
        "{}|{}|{scope_nonce}",
        repo_id.unwrap_or_default(),
        branch.map(PeerId::as_str).unwrap_or_default()
    )
}

pub fn commit_projection_cache_key(
    commit_a: Option<&str>,
    commit_b: &str,
    target: &CommitFileDiffTarget,
) -> String {
    serde_json::to_string(&(commit_a, commit_b, target)).expect("serializable commit diff target")
}

pub fn get_projection(scope: &str, key: &str) -> Option<Arc<DiffProjection>> {
    let mut cache = cache().lock().unwrap_or_else(|error| error.into_inner());
    ensure_scope(&mut cache, scope);
    let index = cache.entries.iter().position(|entry| entry.key == key)?;
    let entry = cache.entries.remove(index)?;
    let projection = entry.projection.clone();
    cache.entries.push_front(entry);
    Some(projection)
}

pub fn put_projection(scope: &str, key: String, projection: Arc<DiffProjection>) {
    let Ok(bytes) = projection_wire_size(&projection) else {
        return;
    };
    if bytes > MAX_DIFF_PROJECTION_BYTES {
        return;
    }
    let mut cache = cache().lock().unwrap_or_else(|error| error.into_inner());
    ensure_scope(&mut cache, scope);
    if let Some(index) = cache.entries.iter().position(|entry| entry.key == key)
        && let Some(old) = cache.entries.remove(index)
    {
        cache.bytes = cache.bytes.saturating_sub(old.bytes);
    }
    cache.bytes = cache.bytes.saturating_add(bytes);
    cache.entries.push_front(CacheEntry {
        key,
        bytes,
        projection,
    });
    while cache.entries.len() > MAX_PROJECTIONS || cache.bytes > MAX_DIFF_PROJECTION_BYTES {
        let Some(removed) = cache.entries.pop_back() else {
            break;
        };
        cache.bytes = cache.bytes.saturating_sub(removed.bytes);
    }
}

pub fn clear_projection_cache() {
    *cache().lock().unwrap_or_else(|error| error.into_inner()) = ProjectionCache::default();
}

fn ensure_scope(cache: &mut ProjectionCache, scope: &str) {
    if cache.scope.as_deref() != Some(scope) {
        cache.entries.clear();
        cache.bytes = 0;
        cache.scope = Some(scope.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::source_control::diff_projection::compute_diff_projection;

    #[test]
    fn cache_is_scope_local_and_lru_bounded() {
        clear_projection_cache();
        for index in 0..5 {
            let projection = Arc::new(
                compute_diff_projection("base".into(), format!("target-{index}")).unwrap(),
            );
            put_projection("scope-a", format!("key-{index}"), projection);
        }
        assert!(get_projection("scope-a", "key-0").is_none());
        assert!(get_projection("scope-a", "key-4").is_some());
        assert!(get_projection("scope-b", "key-4").is_none());
    }
}
