use super::{
    INITIAL_DIFF_PREVIEW_BYTES, INITIAL_DIFF_PREVIEW_LINES, InitialKeyPolicy,
    initial_cached_or_preview, initial_diff_key_policy, preview_input, recompute_with_cache,
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
fn diff_first_viewport_large_single_line_defers_cache_key() {
    let old_content = "a".repeat(INITIAL_DIFF_PREVIEW_BYTES + 1);
    let new_content = "b".repeat(INITIAL_DIFF_PREVIEW_BYTES + 1);

    assert_eq!(
        initial_diff_key_policy(&old_content, &new_content),
        InitialKeyPolicy::DeferUntilFullCompute
    );
}

#[test]
fn diff_first_viewport_large_single_line_preview_is_byte_bounded() {
    let old_content = "a".repeat(INITIAL_DIFF_PREVIEW_BYTES + 1);
    let new_content = "b".repeat(INITIAL_DIFF_PREVIEW_BYTES + 1);
    let initial = initial_cached_or_preview(
        "repo-preview-large-line",
        "path.md",
        &old_content,
        &new_content,
        "unified",
        3,
    );
    let max_left = initial
        .value
        .0
        .0
        .iter()
        .map(|line| line.content.len())
        .max()
        .unwrap_or(0);
    let max_right = initial
        .value
        .0
        .1
        .iter()
        .map(|line| line.content.len())
        .max()
        .unwrap_or(0);

    assert!(!initial.complete);
    assert!(initial.key.is_none());
    assert!(max_left <= INITIAL_DIFF_PREVIEW_BYTES);
    assert!(max_right <= INITIAL_DIFF_PREVIEW_BYTES);
}

#[test]
fn diff_first_viewport_preview_respects_utf8_byte_boundary() {
    let content = "€".repeat((INITIAL_DIFF_PREVIEW_BYTES / "€".len()) + 2);
    let preview = preview_input(&content);

    assert!(preview.len() <= INITIAL_DIFF_PREVIEW_BYTES);
    assert!(content.starts_with(preview));
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
