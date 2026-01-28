// crates/core/src/source_control/diff.rs
//! # Source Control Diff Utilities
//!
//! Provides unified diff generation for committed vs current content.

use similar::TextDiff;

/// Generate a unified diff for a single document.
pub fn unified_diff(old: &str, new: &str, path: &str) -> String {
    let left = if path.is_empty() {
        "a/unknown".to_string()
    } else {
        format!("a/{}", path)
    };
    let right = if path.is_empty() {
        "b/unknown".to_string()
    } else {
        format!("b/{}", path)
    };

    TextDiff::from_lines(old, new)
        .unified_diff()
        .header(&left, &right)
        .to_string()
}
