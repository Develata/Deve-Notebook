use super::super::super::cache::{DiffLines, build_key, cache_get, cache_put};
use super::super::super::model::{DiffAlgorithm, compute_diff_with_meta};

pub fn algo_label(algo: DiffAlgorithm) -> &'static str {
    match algo {
        DiffAlgorithm::Myers => "Myers",
        DiffAlgorithm::PatienceMyers => "Patience+Myers",
    }
}

pub fn initial_with_cache(
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
    let computed = compute_diff_with_meta(old_content, new_content);
    cache_put(key, computed.clone());
    (false, computed)
}

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
    let value = compute_diff_with_meta(old_content, new_content);
    cache_put(key, value.clone());
    (false, value)
}
