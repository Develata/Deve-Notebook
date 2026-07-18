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
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, mpsc};
use std::time::Duration;

pub(crate) struct WorkerInput {
    pub sync: Arc<SyncManager>,
    pub repo_name: String,
    pub repo_id: RepoId,
    pub generation: u64,
    pub repo_root: PathBuf,
    pub backend: Box<dyn FsWatcherBackend>,
    pub command_rx: mpsc::Receiver<WorkerCommand>,
    pub refresh: Option<WatcherRefreshCallback>,
    pub failure: Option<WatcherFailureCallback>,
    pub state: Arc<RwLock<WorkerStateSlot>>,
}

pub(crate) enum WorkerCommand {
    Shutdown,
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
        command_rx,
        refresh,
        failure: failure_callback,
        state,
    } = input;
    let primary = catch_unwind(AssertUnwindSafe(|| {
        consume_loop(
            &sync,
            &repo_name,
            repo_id,
            &repo_root,
            backend.as_mut(),
            &command_rx,
            refresh.as_ref(),
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
    let shutdown = final_state_shutdown(
        &sync,
        &repo_name,
        repo_id,
        &repo_root,
        backend.as_mut(),
        refresh.as_ref(),
    );
    let mut result = combine_primary_and_shutdown(primary, shutdown);
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
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: &Path,
    backend: &mut dyn FsWatcherBackend,
    command_rx: &mpsc::Receiver<WorkerCommand>,
    refresh: Option<&WatcherRefreshCallback>,
) -> Result<(), WatcherFailure> {
    loop {
        match command_rx.try_recv() {
            Ok(WorkerCommand::Shutdown) | Err(mpsc::TryRecvError::Disconnected) => return Ok(()),
            Err(mpsc::TryRecvError::Empty) => {}
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
                    sync,
                    repo_name,
                    repo_id,
                    repo_root,
                    batch.hints(),
                    refresh,
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
                    rescan_and_notify(sync, repo_name, repo_id, repo_root, refresh)
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

fn final_state_shutdown(
    sync: &SyncManager,
    repo_name: &str,
    repo_id: RepoId,
    repo_root: &Path,
    backend: &mut dyn FsWatcherBackend,
    refresh: Option<&WatcherRefreshCallback>,
) -> Result<(), WatcherFailure> {
    // WorkerCommand::Shutdown is only a coordinator request; it is not itself
    // the consumer Stopping cut. This same thread exclusively owns recv and
    // dispatch, so it cannot claim or dispatch another batch while first
    // stopping/joining the producer here. Once stop_backend returns, the thread
    // is the dispatch-quiesced barrier. Only then may the queued suffix be
    // discarded and pending state rebuilt from the exact final filesystem view.
    let mut result = stop_backend(backend);
    append_shutdown_result(&mut result, discard_pending_hints(backend));
    let final_reconcile = match catch_unwind(AssertUnwindSafe(|| {
        rescan_and_notify(sync, repo_name, repo_id, repo_root, refresh)
    })) {
        Ok(result) => result.map_err(|error| {
            WatcherFailure::new(
                WatcherFailurePhase::Reconcile,
                WatcherFailureKind::Scan,
                error.to_string(),
            )
        }),
        Err(panic) => Err(panic_failure(WatcherFailurePhase::Reconcile, panic)),
    };
    append_shutdown_result(&mut result, final_reconcile);
    result
}

fn discard_pending_hints(backend: &dyn FsWatcherBackend) -> Result<(), WatcherFailure> {
    catch_unwind(AssertUnwindSafe(|| backend.discard_pending_hints())).map_err(|panic| {
        WatcherFailure::new(
            WatcherFailurePhase::Shutdown,
            WatcherFailureKind::Panic,
            format!("discard watcher pending hints: {}", panic_message(panic)),
        )
    })
}

fn combine_primary_and_shutdown(
    primary: Result<(), WatcherFailure>,
    shutdown: Result<(), WatcherFailure>,
) -> Result<(), WatcherFailure> {
    match (primary, shutdown) {
        (Err(mut primary), Err(cleanup)) => {
            append_failure(&mut primary, cleanup);
            Err(primary)
        }
        (Err(primary), Ok(())) => Err(primary),
        (Ok(()), shutdown) => shutdown,
    }
}

fn append_shutdown_result(
    result: &mut Result<(), WatcherFailure>,
    next: Result<(), WatcherFailure>,
) {
    let Err(next) = next else {
        return;
    };
    match result {
        Ok(()) => *result = Err(next),
        Err(primary) => append_failure(primary, next),
    }
}

fn append_failure(primary: &mut WatcherFailure, cleanup: WatcherFailure) {
    primary.cleanup.push(cleanup.to_string());
    primary.cleanup.extend(cleanup.cleanup);
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
    repo_root: &Path,
    callback: Option<&WatcherRefreshCallback>,
) -> anyhow::Result<()> {
    let (refreshed_repo_id, refreshed_path) =
        sync.force_dir_refresh_at_root(repo_name, repo_id, repo_root, "")?;
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

#[cfg(test)]
#[path = "worker_shutdown_tests.rs"]
mod shutdown_tests;
