//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 07_network#server-ws-runtime
//!   - 14_commands#cli-commands
//!
//! Host-owned watcher handle collection shared by server startup and the
//! standalone watch command. W4 evolves this owner into WatcherSupervisor.

use deve_core::models::RepoId;
use deve_core::sync::watcher::{
    RepoWatcherHandle, RepoWatcherStart, RepoWatcherWorkerState, WatcherFailure, WatcherStartError,
};
use std::collections::HashSet;

pub(crate) struct OwnedWatcherHandles {
    handles: Vec<RepoWatcherHandle>,
}

impl OwnedWatcherHandles {
    pub(crate) fn start_all(starts: Vec<RepoWatcherStart>) -> Result<Self, WatcherBatchStartError> {
        let mut repo_ids = HashSet::with_capacity(starts.len());
        for start in &starts {
            if !repo_ids.insert(start.repo_id()) {
                return Err(WatcherBatchStartError::DuplicateRepo(start.repo_id()));
            }
        }

        let mut handles = Vec::with_capacity(starts.len());
        for start in starts {
            let repo_id = start.repo_id();
            match RepoWatcherHandle::start(start) {
                Ok(handle) => handles.push(handle),
                Err(source) => {
                    let cleanup = shutdown_handles(&mut handles);
                    return Err(WatcherBatchStartError::Start {
                        repo_id,
                        source,
                        cleanup,
                    });
                }
            }
        }
        Ok(Self { handles })
    }

    pub(crate) fn terminal_failure(&self) -> Option<WatcherFailure> {
        self.handles
            .iter()
            .find_map(|handle| match handle.snapshot().worker_state() {
                RepoWatcherWorkerState::Running => None,
                RepoWatcherWorkerState::Failed(failure) => Some(failure.clone()),
            })
    }

    pub(crate) fn shutdown(mut self) -> Result<(), WatcherCollectionShutdownError> {
        let failures = shutdown_handles(&mut self.handles);
        if failures.is_empty() {
            Ok(())
        } else {
            Err(WatcherCollectionShutdownError { failures })
        }
    }
}

impl Drop for OwnedWatcherHandles {
    fn drop(&mut self) {
        for failure in shutdown_handles(&mut self.handles) {
            tracing::error!(
                error = %failure,
                cleanup = ?failure.cleanup,
                "best-effort watcher collection shutdown failed during Drop"
            );
        }
    }
}

#[derive(Debug)]
pub(crate) enum WatcherBatchStartError {
    DuplicateRepo(RepoId),
    Start {
        repo_id: RepoId,
        source: WatcherStartError,
        cleanup: Vec<WatcherFailure>,
    },
}

impl std::fmt::Display for WatcherBatchStartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateRepo(repo_id) => {
                write!(
                    formatter,
                    "duplicate watcher reservation for repo {repo_id}"
                )
            }
            Self::Start {
                repo_id,
                source,
                cleanup,
            } => {
                write!(
                    formatter,
                    "watcher start failed for repo {repo_id}: {source}"
                )?;
                write_failures(formatter, cleanup)
            }
        }
    }
}

impl std::error::Error for WatcherBatchStartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DuplicateRepo(_) => None,
            Self::Start { source, .. } => Some(source),
        }
    }
}

#[derive(Debug)]
pub(crate) struct WatcherCollectionShutdownError {
    failures: Vec<WatcherFailure>,
}

impl std::fmt::Display for WatcherCollectionShutdownError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("watcher collection shutdown failed")?;
        write_failures(formatter, &self.failures)
    }
}

impl std::error::Error for WatcherCollectionShutdownError {}

fn shutdown_handles(handles: &mut Vec<RepoWatcherHandle>) -> Vec<WatcherFailure> {
    shutdown_reverse(handles, RepoWatcherHandle::shutdown)
}

fn shutdown_reverse<T, E>(
    items: &mut Vec<T>,
    mut shutdown: impl FnMut(T) -> Result<(), E>,
) -> Vec<E> {
    let mut failures = Vec::new();
    while let Some(item) = items.pop() {
        if let Err(error) = shutdown(item) {
            failures.push(error);
        }
    }
    failures
}

fn write_failures(
    formatter: &mut std::fmt::Formatter<'_>,
    failures: &[WatcherFailure],
) -> std::fmt::Result {
    for failure in failures {
        write!(formatter, "; cleanup failure: {failure}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use deve_core::ledger::RepoManager;
    use deve_core::sync::SyncManager;
    use std::sync::Arc;
    use std::time::Duration;

    type WatcherFixture = (
        tempfile::TempDir,
        Arc<RepoManager>,
        Arc<SyncManager>,
        String,
        RepoId,
    );

    fn fixture() -> anyhow::Result<WatcherFixture> {
        let dir = tempfile::tempdir()?;
        let projection_base = dir.path().join("notes");
        std::fs::create_dir_all(&projection_base)?;
        let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))?;
        repo.set_projection_base_for_local_repo("main", &projection_base)?;
        let repo = Arc::new(repo);
        let sync = Arc::new(SyncManager::new_checked(repo.clone())?);
        let info = repo
            .get_repo_info_for(None, Some("main"))?
            .expect("main repo");
        Ok((dir, repo, sync, info.name, info.uuid))
    }

    #[test]
    fn standalone_watch_duplicate_batch_reservation_fails_before_backend_attach()
    -> anyhow::Result<()> {
        let (_dir, repo, sync, repo_name, repo_id) = fixture()?;
        let root = repo.local_repo_workspace_root(&repo_name)?;
        let first = RepoWatcherStart::resolve(sync.clone(), &repo_name, 1)?;
        let second = RepoWatcherStart::resolve(sync, &repo_name, 2)?;
        if root.try_exists()? {
            std::fs::remove_dir_all(root)?;
        }

        let error = match OwnedWatcherHandles::start_all(vec![first, second]) {
            Ok(_) => panic!("duplicate batch reservation must fail"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            WatcherBatchStartError::DuplicateRepo(id) if id == repo_id
        ));
        Ok(())
    }

    #[test]
    fn standalone_watch_partial_batch_start_rolls_back_owned_handles() -> anyhow::Result<()> {
        let (_dir, repo, sync, repo_name, repo_id) = fixture()?;
        let valid = RepoWatcherStart::resolve(sync.clone(), &repo_name, 1)?;
        let invalid = RepoWatcherStart::new(sync, RepoId::new_v4(), &repo_name, 2);

        let error = match OwnedWatcherHandles::start_all(vec![valid, invalid]) {
            Ok(_) => panic!("identity mismatch must fail batch start"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            WatcherBatchStartError::Start { repo_id: failed_id, .. } if failed_id != repo_id
        ));

        let path = repo.local_repo_workspace_path(&repo_name, "notes/after-rollback.md")?;
        std::fs::create_dir_all(path.parent().expect("rollback parent"))?;
        std::fs::write(path, "after rollback")?;
        std::thread::sleep(Duration::from_millis(700));
        assert!(repo.list_pending_fs_in_local_repo(&repo_name)?.is_empty());
        Ok(())
    }

    #[test]
    fn standalone_watch_shutdown_is_reverse_and_continues_after_failure() {
        let mut items = vec![1_u8, 2, 3];
        let mut order = Vec::new();

        let failures = shutdown_reverse(&mut items, |item| {
            order.push(item);
            if item == 2 { Err("two") } else { Ok(()) }
        });

        assert_eq!(order, vec![3, 2, 1]);
        assert_eq!(failures, vec!["two"]);
        assert!(items.is_empty());
    }
}
