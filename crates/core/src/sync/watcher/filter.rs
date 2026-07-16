//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use crate::utils::notegit::is_internal_repo_path;
use notify_debouncer_full::notify::EventKind;

pub(crate) fn allows_repo_path(path: &str) -> bool {
    let normalized = path.trim_matches('/');
    !normalized.is_empty() && normalized.ends_with(".md") && !is_internal_repo_path(normalized)
}

pub(crate) fn allows_repo_dir_path(path: &str) -> bool {
    let normalized = path.trim_matches('/');
    normalized.is_empty() || !is_internal_repo_path(normalized)
}

pub(crate) fn allows_directory_refresh(kind: &EventKind) -> bool {
    !matches!(kind, EventKind::Access(_))
}

#[cfg(test)]
mod tests {
    use super::allows_directory_refresh;
    use notify_debouncer_full::notify::{
        EventKind,
        event::{AccessKind, RemoveKind},
    };

    #[test]
    fn directory_access_does_not_request_refresh() {
        assert!(!allows_directory_refresh(&EventKind::Access(
            AccessKind::Open(notify_debouncer_full::notify::event::AccessMode::Any),
        )));
        assert!(allows_directory_refresh(&EventKind::Remove(
            RemoveKind::Folder,
        )));
    }
}
