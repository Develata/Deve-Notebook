//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::types::{
    RepoWatcherSnapshot, RepoWatcherStart, RepoWatcherWorkerState, WatcherFailure,
    WatcherFailureKind, WatcherFailurePhase, WatcherStartError,
};
use super::{backend, worker};
use crate::models::RepoId;
use std::sync::{Arc, RwLock, mpsc};
use std::thread::JoinHandle;

pub struct RepoWatcherHandle {
    repo_id: RepoId,
    generation: u64,
    state: Arc<RwLock<RepoWatcherWorkerState>>,
    stop_tx: Option<mpsc::Sender<()>>,
    join: Option<JoinHandle<Result<(), WatcherFailure>>>,
}

impl RepoWatcherHandle {
    pub fn start(start: RepoWatcherStart) -> Result<Self, WatcherStartError> {
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
        let info = start
            .sync
            .repo
            .get_repo_info_for(None, Some(&start.repo_name))
            .map_err(|error| {
                start_error(
                    WatcherFailurePhase::Prepare,
                    WatcherFailureKind::Repository,
                    error.to_string(),
                )
            })?
            .ok_or_else(|| {
                start_error(
                    WatcherFailurePhase::Prepare,
                    WatcherFailureKind::Repository,
                    format!("repository missing: {}", start.repo_name),
                )
            })?;
        if start.repo_id != info.uuid {
            return Err(start_error(
                WatcherFailurePhase::Prepare,
                WatcherFailureKind::Repository,
                format!(
                    "repository identity mismatch for {}: expected {}, found {}",
                    start.repo_name, start.repo_id, info.uuid
                ),
            ));
        }
        crate::sync::scan::scan_local_repo(&start.sync.repo, &start.sync.vfs, &start.repo_name)
            .map_err(|error| {
                start_error(
                    WatcherFailurePhase::InitialScan,
                    WatcherFailureKind::Scan,
                    error.to_string(),
                )
            })?;
        let repo_root = start
            .sync
            .repo
            .local_repo_workspace_root(&start.repo_name)
            .and_then(|root| std::fs::canonicalize(&root).map_err(anyhow::Error::from))
            .map_err(|error| {
                start_error(
                    WatcherFailurePhase::Prepare,
                    WatcherFailureKind::Repository,
                    error.to_string(),
                )
            })?;
        let mut backend =
            backend::desktop_backend(&repo_root, start.debounce).map_err(|error| {
                start_error(
                    WatcherFailurePhase::Attach,
                    WatcherFailureKind::Backend,
                    error.to_string(),
                )
            })?;
        let state = Arc::new(RwLock::new(RepoWatcherWorkerState::Running));
        let (stop_tx, stop_rx) = mpsc::channel();
        let worker_state = state.clone();
        let repo_name = start.repo_name;
        let repo_id = info.uuid;
        let generation = start.generation;
        let thread_name = format!("deve-watcher-{}-{generation}", &repo_id.to_string()[..8]);
        let (handoff_tx, handoff_rx) = mpsc::sync_channel::<worker::WorkerInput>(0);
        let join = match std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || match handoff_rx.recv() {
                Ok(input) => worker::run(input),
                Err(error) => Err(WatcherFailure::new(
                    WatcherFailurePhase::WorkerSpawn,
                    WatcherFailureKind::Coordination,
                    error.to_string(),
                )),
            }) {
            Ok(join) => join,
            Err(error) => {
                let mut failure = WatcherFailure::new(
                    WatcherFailurePhase::WorkerSpawn,
                    WatcherFailureKind::Thread,
                    error.to_string(),
                );
                if let Err(cleanup) = worker::stop_backend(backend.as_mut()) {
                    failure.cleanup.push(cleanup.to_string());
                }
                return Err(WatcherStartError::new(failure));
            }
        };
        let input = worker::WorkerInput {
            sync: start.sync,
            repo_name,
            repo_id,
            repo_root,
            backend,
            stop_rx,
            refresh: start.refresh,
            state: worker_state,
        };
        if let Err(error) = handoff_tx.send(input) {
            let mut input = error.0;
            let mut failure = match join.join() {
                Ok(Err(failure)) => failure,
                Ok(Ok(())) => WatcherFailure::new(
                    WatcherFailurePhase::WorkerSpawn,
                    WatcherFailureKind::Coordination,
                    "watcher worker exited before runtime handoff",
                ),
                Err(panic) => WatcherFailure::new(
                    WatcherFailurePhase::WorkerSpawn,
                    WatcherFailureKind::Panic,
                    worker::panic_message(panic),
                ),
            };
            if let Err(cleanup) = worker::stop_backend(input.backend.as_mut()) {
                failure.cleanup.push(cleanup.to_string());
            }
            return Err(WatcherStartError::new(failure));
        }
        Ok(Self {
            repo_id,
            generation,
            state,
            stop_tx: Some(stop_tx),
            join: Some(join),
        })
    }

    pub fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn snapshot(&self) -> RepoWatcherSnapshot {
        let worker_state = match self.state.read() {
            Ok(state) => state.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        RepoWatcherSnapshot::new(self.repo_id, self.generation, worker_state)
    }

    pub fn shutdown(mut self) -> Result<(), WatcherFailure> {
        self.shutdown_inner()
    }

    fn shutdown_inner(&mut self) -> Result<(), WatcherFailure> {
        let send_error = self
            .stop_tx
            .take()
            .and_then(|sender| sender.send(()).err())
            .map(|error| error.to_string());
        let Some(join) = self.join.take() else {
            return send_error.map_or(Ok(()), |error| {
                Err(WatcherFailure::new(
                    WatcherFailurePhase::Shutdown,
                    WatcherFailureKind::Coordination,
                    error,
                ))
            });
        };
        let result = match join.join() {
            Ok(result) => result,
            Err(_) => Err(WatcherFailure::new(
                WatcherFailurePhase::Shutdown,
                WatcherFailureKind::Panic,
                "watcher worker panicked outside the failure boundary",
            )),
        };
        match (result, send_error) {
            (Err(failure), Some(cleanup)) => Err(failure.with_cleanup(cleanup)),
            (Err(failure), None) => Err(failure),
            (Ok(()), Some(error)) => Err(WatcherFailure::new(
                WatcherFailurePhase::Shutdown,
                WatcherFailureKind::Coordination,
                error,
            )),
            (Ok(()), None) => Ok(()),
        }
    }
}

impl Drop for RepoWatcherHandle {
    fn drop(&mut self) {
        if self.join.is_none() {
            return;
        }
        tracing::warn!(
            repo_id = %self.repo_id,
            generation = self.generation,
            "RepoWatcherHandle dropped without explicit shutdown"
        );
        if let Err(error) = self.shutdown_inner() {
            tracing::error!(
                repo_id = %self.repo_id,
                generation = self.generation,
                error = %error,
                cleanup = ?error.cleanup,
                "best-effort watcher shutdown failed during Drop"
            );
        }
    }
}

fn start_error(
    phase: WatcherFailurePhase,
    kind: WatcherFailureKind,
    primary: impl Into<String>,
) -> WatcherStartError {
    WatcherStartError::new(WatcherFailure::new(phase, kind, primary))
}
