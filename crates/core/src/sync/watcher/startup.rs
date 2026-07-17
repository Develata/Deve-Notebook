//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::backend::{self, FsWatcherBackend, StartupHandoff};
use super::panic_message;
use super::types::{
    RepoWatcherStart, WatcherFailure, WatcherFailureKind, WatcherFailurePhase, WatcherStartError,
};
use super::worker::{self, WorkerInput, WorkerStateSlot};
use crate::models::RepoId;
use crate::sync::SyncManager;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::{Arc, RwLock, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

const MAX_STARTUP_SCAN_PASSES: usize = 3;

pub(super) struct StartedWatcher {
    pub repo_id: RepoId,
    pub generation: u64,
    pub state: Arc<RwLock<WorkerStateSlot>>,
    pub stop_tx: mpsc::Sender<()>,
    pub join: JoinHandle<Result<(), WatcherFailure>>,
}

pub(super) fn start(start: RepoWatcherStart) -> Result<StartedWatcher, WatcherStartError> {
    start_with(
        start,
        backend::desktop_backend,
        |sync, repo_name, repo_root| {
            crate::sync::scan::scan_local_repo_at_root(&sync.repo, &sync.vfs, repo_name, repo_root)
        },
    )
}

fn start_with<A, S>(
    start: RepoWatcherStart,
    attach: A,
    mut scan: S,
) -> Result<StartedWatcher, WatcherStartError>
where
    A: FnOnce(&Path, Duration, u64) -> Result<Box<dyn FsWatcherBackend>, WatcherFailure>,
    S: FnMut(&Arc<SyncManager>, &str, &Path) -> anyhow::Result<()>,
{
    validate_start(&start)?;
    let info = start
        .sync
        .repo
        .get_repo_info_for(None, Some(&start.repo_name))
        .map_err(|error| repository_start_error(error.to_string()))?
        .ok_or_else(|| {
            repository_start_error(format!("repository missing: {}", start.repo_name))
        })?;
    if start.repo_id != info.uuid {
        return Err(repository_start_error(format!(
            "repository identity mismatch for {}: expected {}, found {}",
            start.repo_name, start.repo_id, info.uuid
        )));
    }
    let repo_root = prepare_root(&start)?;
    let mut backend =
        attach(&repo_root, start.debounce, start.generation).map_err(WatcherStartError::new)?;

    let state = Arc::new(RwLock::new(WorkerStateSlot::running(start.generation)));
    let (stop_tx, stop_rx) = mpsc::channel();
    let worker_state = state.clone();
    let repo_id = info.uuid;
    let generation = start.generation;
    let thread_name = format!("deve-watcher-{}-{generation}", &repo_id.to_string()[..8]);
    let (handoff_tx, handoff_rx) = mpsc::sync_channel::<WorkerInput>(0);
    let join = match std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || match handoff_rx.recv() {
            Ok(input) => worker::run(input),
            Err(_) => Ok(()),
        }) {
        Ok(join) => join,
        Err(error) => {
            let failure = WatcherFailure::new(
                WatcherFailurePhase::WorkerSpawn,
                WatcherFailureKind::Thread,
                error.to_string(),
            );
            return Err(WatcherStartError::new(stop_attached_backend(
                failure,
                backend.as_mut(),
            )));
        }
    };

    if let Err(failure) = run_capture_first_scans(
        &start.sync,
        &start.repo_name,
        &repo_root,
        backend.as_ref(),
        &mut scan,
    ) {
        return Err(WatcherStartError::new(abort_before_handoff(
            failure, backend, handoff_tx, join,
        )));
    }

    let input = WorkerInput {
        sync: start.sync,
        repo_name: start.repo_name,
        repo_id,
        generation,
        repo_root,
        backend,
        stop_rx,
        refresh: start.refresh,
        failure: start.failure,
        state: worker_state,
    };
    if let Err(error) = handoff_tx.send(input) {
        let mut input = error.0;
        let failure = match join.join() {
            Ok(Err(failure)) => failure,
            Ok(Ok(())) => WatcherFailure::new(
                WatcherFailurePhase::WorkerSpawn,
                WatcherFailureKind::Coordination,
                "watcher worker exited before runtime handoff",
            ),
            Err(panic) => WatcherFailure::new(
                WatcherFailurePhase::WorkerSpawn,
                WatcherFailureKind::Panic,
                panic_message(panic),
            ),
        };
        return Err(WatcherStartError::new(stop_attached_backend(
            failure,
            input.backend.as_mut(),
        )));
    }

    Ok(StartedWatcher {
        repo_id,
        generation,
        state,
        stop_tx,
        join,
    })
}

fn validate_start(start: &RepoWatcherStart) -> Result<(), WatcherStartError> {
    if start.generation == 0 {
        return Err(start_error(
            WatcherFailurePhase::Prepare,
            WatcherFailureKind::Configuration,
            "watcher generation must be non-zero",
        ));
    }
    if start.debounce.is_zero() {
        return Err(start_error(
            WatcherFailurePhase::Prepare,
            WatcherFailureKind::Configuration,
            "watcher debounce window must be non-zero",
        ));
    }
    Ok(())
}

fn prepare_root(start: &RepoWatcherStart) -> Result<std::path::PathBuf, WatcherStartError> {
    let root = start
        .sync
        .repo
        .local_repo_workspace_root(&start.repo_name)
        .map_err(|error| repository_start_error(error.to_string()))?;
    std::fs::create_dir_all(&root)
        .and_then(|()| std::fs::canonicalize(&root))
        .map_err(|error| repository_start_error(error.to_string()))
}

fn run_capture_first_scans<S>(
    sync: &Arc<SyncManager>,
    repo_name: &str,
    repo_root: &Path,
    backend: &dyn FsWatcherBackend,
    scan: &mut S,
) -> Result<(), WatcherFailure>
where
    S: FnMut(&Arc<SyncManager>, &str, &Path) -> anyhow::Result<()>,
{
    for pass in 1..=MAX_STARTUP_SCAN_PASSES {
        let token = catch_unwind(AssertUnwindSafe(|| backend.begin_startup_scan()))
            .map_err(|panic| startup_panic("begin startup scan", panic))??;
        catch_unwind(AssertUnwindSafe(|| scan(sync, repo_name, repo_root)))
            .map_err(|panic| startup_panic("startup full scan", panic))?
            .map_err(|error| {
                WatcherFailure::new(
                    WatcherFailurePhase::InitialScan,
                    WatcherFailureKind::Scan,
                    error.to_string(),
                )
            })?;
        let handoff = catch_unwind(AssertUnwindSafe(|| backend.complete_startup_scan(token)))
            .map_err(|panic| startup_panic("complete startup scan", panic))??;
        match handoff {
            StartupHandoff::Running => return Ok(()),
            StartupHandoff::Dirty if pass < MAX_STARTUP_SCAN_PASSES => {}
            StartupHandoff::Dirty => {
                return Err(WatcherFailure::new(
                    WatcherFailurePhase::InitialScan,
                    WatcherFailureKind::StartupChurn,
                    format!(
                        "workspace changed during all {MAX_STARTUP_SCAN_PASSES} startup scan passes"
                    ),
                ));
            }
        }
    }
    unreachable!("startup scan loop returns on clean handoff or final dirty pass")
}

fn abort_before_handoff(
    failure: WatcherFailure,
    mut backend: Box<dyn FsWatcherBackend>,
    handoff_tx: mpsc::SyncSender<WorkerInput>,
    join: JoinHandle<Result<(), WatcherFailure>>,
) -> WatcherFailure {
    let mut failure = stop_attached_backend(failure, backend.as_mut());
    drop(handoff_tx);
    match join.join() {
        Ok(Ok(())) => {}
        Ok(Err(cleanup)) => append_cleanup(&mut failure, cleanup),
        Err(panic) => failure.cleanup.push(format!(
            "watcher startup worker join panic: {}",
            panic_message(panic)
        )),
    }
    failure
}

fn stop_attached_backend(
    mut failure: WatcherFailure,
    backend: &mut dyn FsWatcherBackend,
) -> WatcherFailure {
    if let Err(cleanup) = worker::stop_backend(backend) {
        append_cleanup(&mut failure, cleanup);
    }
    failure
}

fn append_cleanup(failure: &mut WatcherFailure, cleanup: WatcherFailure) {
    failure.cleanup.push(cleanup.to_string());
    failure.cleanup.extend(cleanup.cleanup);
}

fn repository_start_error(primary: impl Into<String>) -> WatcherStartError {
    start_error(
        WatcherFailurePhase::Prepare,
        WatcherFailureKind::Repository,
        primary,
    )
}

fn start_error(
    phase: WatcherFailurePhase,
    kind: WatcherFailureKind,
    primary: impl Into<String>,
) -> WatcherStartError {
    WatcherStartError::new(WatcherFailure::new(phase, kind, primary))
}

fn startup_panic(context: &str, panic: Box<dyn std::any::Any + Send>) -> WatcherFailure {
    WatcherFailure::new(
        WatcherFailurePhase::InitialScan,
        WatcherFailureKind::Panic,
        format!("{context}: {}", panic_message(panic)),
    )
}

#[cfg(test)]
#[path = "startup_tests.rs"]
mod tests;
