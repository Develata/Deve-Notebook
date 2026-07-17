//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::backend::{BackendSignal, FsWatcherBackend};
use super::{
    RepoWatcherWorkerState, WatcherFailure, WatcherFailureCallback, WatcherFailureKind,
    WatcherFailurePhase, WatcherRefresh, WatcherRefreshCallback, dispatch, panic_message,
};
use crate::models::RepoId;
use crate::sync::SyncManager;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::{Arc, RwLock, mpsc};
use std::time::Duration;

pub(crate) struct WorkerInput {
    pub sync: Arc<SyncManager>,
    pub repo_name: String,
    pub repo_id: RepoId,
    pub generation: u64,
    pub repo_root: PathBuf,
    pub backend: Box<dyn FsWatcherBackend>,
    pub stop_rx: mpsc::Receiver<()>,
    pub refresh: Option<WatcherRefreshCallback>,
    pub failure: Option<WatcherFailureCallback>,
    pub state: Arc<RwLock<WorkerStateSlot>>,
}

pub(crate) struct WorkerStateSlot {
    generation: u64,
    state: RepoWatcherWorkerState,
}

impl WorkerStateSlot {
    pub(crate) fn running(generation: u64) -> Self {
        Self {
            generation,
            state: RepoWatcherWorkerState::Running,
        }
    }

    pub(crate) fn snapshot(&self, generation: u64) -> Option<RepoWatcherWorkerState> {
        (self.generation == generation).then(|| self.state.clone())
    }
}

pub(crate) fn run(input: WorkerInput) -> Result<(), WatcherFailure> {
    let WorkerInput {
        sync,
        repo_name,
        repo_id,
        generation,
        repo_root,
        mut backend,
        stop_rx,
        refresh,
        failure: failure_callback,
        state,
    } = input;
    let primary = catch_unwind(AssertUnwindSafe(|| {
        consume_loop(
            sync,
            &repo_name,
            repo_id,
            repo_root,
            backend.as_mut(),
            stop_rx,
            refresh,
        )
    }))
    .unwrap_or_else(|panic| {
        Err(WatcherFailure::new(
            WatcherFailurePhase::Worker,
            WatcherFailureKind::Panic,
            panic_message(panic),
        ))
    });
    let primary_failed = primary.is_err();
    let callback_cleanup = if let Err(primary_failure) = &primary {
        publish_failure_cut(
            &state,
            generation,
            primary_failure,
            failure_callback.as_ref(),
        )
    } else {
        None
    };
    let cleanup = stop_backend(backend.as_mut());
    let mut result = match (primary, cleanup) {
        (Err(failure), Err(cleanup)) => Err(failure.with_cleanup(cleanup.to_string())),
        (Err(failure), Ok(())) => Err(failure),
        (Ok(()), Err(cleanup)) => Err(cleanup),
        (Ok(()), Ok(())) => Ok(()),
    };
    if let Some(callback_cleanup) = callback_cleanup
        && let Err(failure) = &mut result
    {
        failure.cleanup.push(callback_cleanup);
    }
    if !primary_failed
        && let Err(failure) = &result
        && let Some(callback_cleanup) =
            publish_failure_cut(&state, generation, failure, failure_callback.as_ref())
        && let Err(current) = &mut result
    {
        current.cleanup.push(callback_cleanup);
    }
    if let Err(failure) = &result {
        replace_state(
            &state,
            generation,
            RepoWatcherWorkerState::Failed(failure.clone()),
        );
        tracing::error!(
            %repo_id,
            generation,
            phase = ?failure.phase,
            kind = ?failure.kind,
            primary = %failure.primary,
            cleanup = ?failure.cleanup,
            "workspace ingestion watcher failed"
        );
    }
    result
}

fn publish_failure_cut(
    state: &RwLock<WorkerStateSlot>,
    generation: u64,
    failure: &WatcherFailure,
    callback: Option<&WatcherFailureCallback>,
) -> Option<String> {
    replace_state(
        state,
        generation,
        RepoWatcherWorkerState::Failed(failure.clone()),
    );
    callback.and_then(|callback| {
        catch_unwind(AssertUnwindSafe(|| callback(failure.clone())))
            .err()
            .map(|panic| {
                format!(
                    "watcher failure callback panicked: {}",
                    panic_message(panic)
                )
            })
    })
}

fn consume_loop(
    sync: Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: PathBuf,
    backend: &mut dyn FsWatcherBackend,
    stop_rx: mpsc::Receiver<()>,
    refresh: Option<WatcherRefreshCallback>,
) -> Result<(), WatcherFailure> {
    loop {
        if stop_rx.try_recv().is_ok() {
            return Ok(());
        }
        let signal = catch_unwind(AssertUnwindSafe(|| {
            backend.recv(Duration::from_millis(100))
        }))
        .map_err(|panic| panic_failure(WatcherFailurePhase::Receive, panic))?
        .map_err(|error| {
            WatcherFailure::new(
                WatcherFailurePhase::Receive,
                WatcherFailureKind::Backend,
                error.to_string(),
            )
        })?;
        match signal {
            Some(BackendSignal::Hints(batch)) => catch_unwind(AssertUnwindSafe(|| {
                dispatch::dispatch_batch(
                    &sync,
                    repo_name,
                    repo_id,
                    &repo_root,
                    batch.hints(),
                    refresh.as_ref(),
                )
            }))
            .map_err(|panic| panic_failure(WatcherFailurePhase::Dispatch, panic))?
            .map_err(|error| {
                WatcherFailure::new(
                    WatcherFailurePhase::Dispatch,
                    WatcherFailureKind::Dispatch,
                    error.to_string(),
                )
            })?,
            Some(BackendSignal::Reconcile(token)) => {
                catch_unwind(AssertUnwindSafe(|| {
                    rescan_and_notify(&sync, repo_name, repo_id, refresh.as_ref())
                }))
                .map_err(|panic| panic_failure(WatcherFailurePhase::Reconcile, panic))?
                .map_err(|error| {
                    WatcherFailure::new(
                        WatcherFailurePhase::Reconcile,
                        WatcherFailureKind::Scan,
                        error.to_string(),
                    )
                })?;
                let _clean =
                    catch_unwind(AssertUnwindSafe(|| backend.complete_reconcile(token)))
                        .map_err(|panic| panic_failure(WatcherFailurePhase::Reconcile, panic))?;
            }
            Some(BackendSignal::Terminal(failure)) => return Err(failure),
            None => {}
        }
    }
}

pub(crate) fn stop_backend(backend: &mut dyn FsWatcherBackend) -> Result<(), WatcherFailure> {
    catch_unwind(AssertUnwindSafe(|| backend.stop()))
        .map_err(|panic| panic_failure(WatcherFailurePhase::Shutdown, panic))?
        .map_err(|error| {
            WatcherFailure::new(
                WatcherFailurePhase::Shutdown,
                WatcherFailureKind::Backend,
                error.to_string(),
            )
        })
}

fn rescan_and_notify(
    sync: &SyncManager,
    repo_name: &str,
    repo_id: RepoId,
    callback: Option<&WatcherRefreshCallback>,
) -> anyhow::Result<()> {
    let (refreshed_repo_id, refreshed_path) = sync.force_dir_refresh(repo_name, repo_id, "")?;
    if let Some(callback) = callback {
        callback(WatcherRefresh::new(
            refreshed_repo_id,
            refreshed_path,
            super::WatcherRefreshKind::DirectoryChanged,
            false,
        ));
    }
    Ok(())
}

fn replace_state(
    state: &RwLock<WorkerStateSlot>,
    generation: u64,
    next: RepoWatcherWorkerState,
) -> bool {
    let mut state = match state.write() {
        Ok(state) => state,
        Err(poisoned) => poisoned.into_inner(),
    };
    if state.generation != generation {
        return false;
    }
    state.state = next;
    true
}

fn panic_failure(phase: WatcherFailurePhase, panic: Box<dyn Any + Send>) -> WatcherFailure {
    WatcherFailure::new(phase, WatcherFailureKind::Panic, panic_message(panic))
}

#[cfg(test)]
#[path = "worker_tests.rs"]
mod tests;
