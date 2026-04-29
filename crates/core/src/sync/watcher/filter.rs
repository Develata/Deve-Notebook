//! plan_ref:
//!   - 04_storage#watcher-contract

use crate::utils::notegit::is_internal_repo_path;

pub(crate) fn allows_repo_path(path: &str) -> bool {
    let normalized = path.trim_matches('/');
    !normalized.is_empty() && normalized.ends_with(".md") && !is_internal_repo_path(normalized)
}

pub(crate) fn allows_repo_dir_path(path: &str) -> bool {
    let normalized = path.trim_matches('/');
    !normalized.is_empty() && !is_internal_repo_path(normalized)
}
