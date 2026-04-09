//! plan_ref:
//!   - 04_storage.md §8. Watcher Contract

use crate::utils::notegit::is_internal_repo_path;

pub(crate) fn allows_repo_path(path: &str) -> bool {
    let normalized = path.trim_matches('/');
    !normalized.is_empty()
        && normalized.ends_with(".md")
        && !normalized.starts_with(".notegit/")
        && !is_internal_repo_path(normalized)
}
