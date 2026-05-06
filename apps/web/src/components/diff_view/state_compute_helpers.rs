//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 03_rendering#large-document-runtime
//!
use super::super::super::cache::{DiffLines, build_key, cache_get, cache_put};
#[cfg(test)]
use super::super::super::model::create_diff_chunk_job;
use super::super::super::model::{DiffAlgorithm, compute_diff_preview_with_meta};
use super::super::super::unified::DIFF_VIEWPORT_CHUNK_SIZE;

pub const INITIAL_DIFF_PREVIEW_LINES: usize = DIFF_VIEWPORT_CHUNK_SIZE * 2;

pub struct InitialDiff {
    pub key: Option<String>,
    pub cache_hit: bool,
    pub complete: bool,
    pub value: (DiffLines, DiffAlgorithm),
}

pub fn initial_diff_key_policy(old_content: &str, new_content: &str) -> InitialKeyPolicy {
    if exceeds_preview_window(old_content) || exceeds_preview_window(new_content) {
        InitialKeyPolicy::DeferUntilFullCompute
    } else {
        InitialKeyPolicy::BuildSynchronously
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialKeyPolicy {
    BuildSynchronously,
    DeferUntilFullCompute,
}

pub fn algo_label(algo: DiffAlgorithm) -> &'static str {
    match algo {
        DiffAlgorithm::Myers => "Myers",
        DiffAlgorithm::PatienceMyers => "Patience+Myers",
    }
}

pub fn initial_cached_or_preview(
    repo_scope: &str,
    path: &str,
    old_content: &str,
    new_content: &str,
    mode: &str,
    context_lines: usize,
) -> InitialDiff {
    let preview = preview_diff(old_content, new_content);
    if initial_diff_key_policy(old_content, new_content) == InitialKeyPolicy::DeferUntilFullCompute
    {
        return InitialDiff {
            key: None,
            cache_hit: false,
            complete: false,
            value: preview,
        };
    }

    let key = build_key(
        repo_scope,
        path,
        old_content,
        new_content,
        mode,
        context_lines,
    );
    if let Some(cached) = cache_get(&key) {
        return InitialDiff {
            key: Some(key),
            cache_hit: true,
            complete: true,
            value: cached,
        };
    }
    cache_put(key.clone(), preview.clone());
    InitialDiff {
        key: Some(key),
        cache_hit: false,
        complete: true,
        value: preview,
    }
}

pub fn preview_diff(old_content: &str, new_content: &str) -> (DiffLines, DiffAlgorithm) {
    compute_diff_preview_with_meta(old_content, new_content, INITIAL_DIFF_PREVIEW_LINES)
}

fn exceeds_preview_window(content: &str) -> bool {
    content.lines().nth(INITIAL_DIFF_PREVIEW_LINES).is_some()
}

#[cfg(test)]
pub fn recompute_with_cache(
    repo_scope: &str,
    path: &str,
    old_content: &str,
    new_content: &str,
    mode: &str,
    context_lines: usize,
) -> (bool, (DiffLines, DiffAlgorithm)) {
    let key = build_key(
        repo_scope,
        path,
        old_content,
        new_content,
        mode,
        context_lines,
    );
    if let Some(cached) = cache_get(&key) {
        return (true, cached);
    }
    let mut job = create_diff_chunk_job(old_content.to_string(), new_content.to_string());
    while !job.step() {}
    let value = job.finish();
    cache_put(key, value.clone());
    (false, value)
}

pub fn cache_completed_diff(key: String, value: (DiffLines, DiffAlgorithm)) {
    cache_put(key, value);
}

#[cfg(test)]
mod tests {
    use super::{
        INITIAL_DIFF_PREVIEW_LINES, InitialKeyPolicy, initial_cached_or_preview,
        initial_diff_key_policy, recompute_with_cache,
    };

    fn long_lines(prefix: &str) -> String {
        (0..3_000)
            .map(|i| format!("{prefix}-{i}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn diff_first_viewport_initial_cache_miss_uses_preview() {
        let old_content = long_lines("old-preview-miss");
        let new_content = long_lines("new-preview-miss");
        let initial = initial_cached_or_preview(
            "repo-preview-miss",
            "path.md",
            &old_content,
            &new_content,
            "unified",
            3,
        );

        assert!(!initial.cache_hit);
        assert!(!initial.complete);
        assert!(initial.key.is_none());
        assert!(initial.value.0.0.len() <= INITIAL_DIFF_PREVIEW_LINES * 2);
    }

    #[test]
    fn diff_first_viewport_long_initial_defers_cache_key() {
        let old_content = long_lines("old-key-policy");
        let new_content = long_lines("new-key-policy");

        assert_eq!(
            initial_diff_key_policy(&old_content, &new_content),
            InitialKeyPolicy::DeferUntilFullCompute
        );
    }

    #[test]
    fn diff_first_viewport_short_initial_builds_cache_key() {
        assert_eq!(
            initial_diff_key_policy("a\nb\nc", "a\nb2\nc"),
            InitialKeyPolicy::BuildSynchronously
        );
    }

    #[test]
    fn diff_first_viewport_initial_short_cache_hit_is_complete() {
        let old_content = "a\nb\nc".to_string();
        let new_content = "a\nb2\nc".to_string();
        let _ = recompute_with_cache(
            "repo-preview-hit",
            "path.md",
            &old_content,
            &new_content,
            "unified",
            3,
        );
        let initial = initial_cached_or_preview(
            "repo-preview-hit",
            "path.md",
            &old_content,
            &new_content,
            "unified",
            3,
        );

        assert!(initial.cache_hit);
        assert!(initial.complete);
        assert!(initial.key.is_some());
    }

    #[test]
    fn diff_first_viewport_initial_short_cache_miss_is_complete() {
        let initial = initial_cached_or_preview(
            "repo-preview-short-miss",
            "path.md",
            "a\nb\nc",
            "a\nb2\nc",
            "unified",
            3,
        );

        assert!(!initial.cache_hit);
        assert!(initial.complete);
        assert!(initial.key.is_some());
    }
}
