//! plan_ref:
//!   - 03_storage/watcher#watcher-contract

use super::startup;
use super::types::{
    RepoWatcherSnapshot, RepoWatcherStart, RepoWatcherWorkerState, WatcherFailure,
    WatcherFailureKind, WatcherFailurePhase, WatcherStartError,
};
use super::worker;
use crate::models::RepoId;
use std::sync::{Arc, RwLock, mpsc};
use std::thread::JoinHandle;

pub struct RepoWatcherHandle {
    repo_id: RepoId,
    generation: u64,
    state: Arc<RwLock<worker::WorkerStateSlot>>,
    stop_tx: Option<mpsc::Sender<()>>,
    join: Option<JoinHandle<Result<(), WatcherFailure>>>,
}

impl RepoWatcherHandle {
    pub fn start(start: RepoWatcherStart) -> Result<Self, WatcherStartError> {
        let started = startup::start(start)?;
        Ok(Self {
            repo_id: started.repo_id,
            generation: started.generation,
            state: started.state,
            stop_tx: Some(started.stop_tx),
            join: Some(started.join),
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
            Ok(state) => state.snapshot(self.generation),
            Err(poisoned) => poisoned.into_inner().snapshot(self.generation),
        }
        .unwrap_or_else(|| {
            RepoWatcherWorkerState::Failed(WatcherFailure::new(
                WatcherFailurePhase::Worker,
                WatcherFailureKind::Coordination,
                "watcher worker state generation mismatch",
            ))
        });
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
            Err(panic) => Err(WatcherFailure::new(
                WatcherFailurePhase::Shutdown,
                WatcherFailureKind::Panic,
                format!(
                    "watcher worker panicked outside the failure boundary: {}",
                    super::panic_message(panic)
                ),
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
