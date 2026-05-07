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
pub const INITIAL_DIFF_PREVIEW_BYTES: usize = 64 * 1024;

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
    let old_preview = preview_input(old_content);
    let new_preview = preview_input(new_content);
    compute_diff_preview_with_meta(old_preview, new_preview, INITIAL_DIFF_PREVIEW_LINES)
}

fn exceeds_preview_window(content: &str) -> bool {
    content.len() > INITIAL_DIFF_PREVIEW_BYTES
        || content.lines().nth(INITIAL_DIFF_PREVIEW_LINES).is_some()
}

fn preview_input(content: &str) -> &str {
    let mut end = 0;
    let mut newline_count = 0;
    for (idx, ch) in content.char_indices() {
        let next_end = idx + ch.len_utf8();
        if next_end > INITIAL_DIFF_PREVIEW_BYTES {
            break;
        }
        end = next_end;
        if ch == '\n' {
            newline_count += 1;
            if newline_count >= INITIAL_DIFF_PREVIEW_LINES {
                break;
            }
        }
    }
    if end == content.len() {
        content
    } else {
        &content[..end]
    }
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
#[path = "state_compute_helpers_test.rs"]
mod tests;
