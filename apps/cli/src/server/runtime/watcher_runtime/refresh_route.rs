//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#repo-health-and-repair
//!
//! Generation-local refresh coalescing for lifecycle transitions.

use deve_core::sync::watcher::{WatcherRefresh, WatcherRefreshKind};

#[derive(Default)]
pub(super) struct DeferredRefresh {
    refresh: Option<WatcherRefresh>,
}

impl DeferredRefresh {
    pub(super) fn push(&mut self, refresh: WatcherRefresh) {
        self.refresh = Some(match self.refresh.take() {
            None => refresh,
            Some(previous) => coalesce(previous, refresh),
        });
    }

    pub(super) fn take(&mut self) -> Option<WatcherRefresh> {
        self.refresh.take()
    }

    pub(super) fn clear(&mut self) {
        self.refresh = None;
    }
}

fn coalesce(previous: WatcherRefresh, next: WatcherRefresh) -> WatcherRefresh {
    debug_assert_eq!(previous.repo_id(), next.repo_id());
    if previous == next {
        return previous;
    }

    WatcherRefresh::new(
        previous.repo_id(),
        ".",
        WatcherRefreshKind::DirectoryChanged,
        previous.has_conflict() || next.has_conflict(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::models::RepoId;

    #[test]
    fn different_refreshes_coalesce_to_one_repo_refresh() {
        let repo_id = RepoId::new_v4();
        let mut deferred = DeferredRefresh::default();
        deferred.push(WatcherRefresh::new(
            repo_id,
            "a.md",
            WatcherRefreshKind::Added,
            false,
        ));
        deferred.push(WatcherRefresh::new(
            repo_id,
            "b.md",
            WatcherRefreshKind::Modified,
            true,
        ));

        let refresh = deferred.take().expect("coalesced refresh");
        assert_eq!(refresh.repo_id(), repo_id);
        assert_eq!(refresh.path(), ".");
        assert_eq!(refresh.kind(), WatcherRefreshKind::DirectoryChanged);
        assert!(refresh.has_conflict());
        assert!(deferred.take().is_none());
    }
}
