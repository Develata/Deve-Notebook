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
use std::time::Duration;

pub struct RepoWatcherHandle {
    repo_id: RepoId,
    generation: u64,
    state: Arc<RwLock<worker::WorkerStateSlot>>,
    command_tx: Option<mpsc::SyncSender<worker::WorkerCommand>>,
    join: Option<JoinHandle<Result<(), WatcherFailure>>>,
}

impl RepoWatcherHandle {
    pub fn start(start: RepoWatcherStart) -> Result<Self, WatcherStartError> {
        let started = startup::start(start)?;
        Ok(Self {
            repo_id: started.repo_id,
            generation: started.generation,
            state: started.state,
            command_tx: Some(started.command_tx),
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

    pub fn shutdown_bounded(mut self, timeout: Duration) -> Result<(), WatcherFailure> {
        let send_error = self.command_tx.take().and_then(|sender| {
            match sender.try_send(worker::WorkerCommand::Shutdown) {
                Ok(()) | Err(mpsc::TrySendError::Full(_)) => None,
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    Some("watcher shutdown command channel disconnected".to_string())
                }
            }
        });
        let Some(join) = self.join.take() else {
            return combine_shutdown_result(Ok(()), send_error);
        };
        let (result_tx, result_rx) = mpsc::sync_channel(1);
        let waiter = std::thread::Builder::new()
            .name("deve-watcher-shutdown-waiter".to_string())
            .spawn(move || {
                let _ = result_tx.send(join_worker(join));
            });
        if waiter.is_err() {
            return combine_shutdown_result(
                Err(WatcherFailure::new(
                    WatcherFailurePhase::Shutdown,
                    WatcherFailureKind::Thread,
                    "watcher shutdown waiter could not start",
                )),
                send_error,
            );
        }
        match result_rx.recv_timeout(timeout) {
            Ok(result) => combine_shutdown_result(result, send_error),
            Err(mpsc::RecvTimeoutError::Timeout) => combine_shutdown_result(
                Err(WatcherFailure::new(
                    WatcherFailurePhase::Shutdown,
                    WatcherFailureKind::Coordination,
                    "watcher shutdown deadline exceeded",
                )),
                send_error,
            ),
            Err(mpsc::RecvTimeoutError::Disconnected) => combine_shutdown_result(
                Err(WatcherFailure::new(
                    WatcherFailurePhase::Shutdown,
                    WatcherFailureKind::Coordination,
                    "watcher shutdown waiter result unavailable",
                )),
                send_error,
            ),
        }
    }

    fn shutdown_inner(&mut self) -> Result<(), WatcherFailure> {
        let send_error = self
            .command_tx
            .take()
            .and_then(|sender| sender.send(worker::WorkerCommand::Shutdown).err())
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
        combine_shutdown_result(join_worker(join), send_error)
    }
}

fn join_worker(join: JoinHandle<Result<(), WatcherFailure>>) -> Result<(), WatcherFailure> {
    match join.join() {
        Ok(result) => result,
        Err(panic) => Err(WatcherFailure::new(
            WatcherFailurePhase::Shutdown,
            WatcherFailureKind::Panic,
            format!(
                "watcher worker panicked outside the failure boundary: {}",
                super::panic_message(panic)
            ),
        )),
    }
}

fn combine_shutdown_result(
    result: Result<(), WatcherFailure>,
    send_error: Option<String>,
) -> Result<(), WatcherFailure> {
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

#[cfg(test)]
mod tests {
    use super::RepoWatcherHandle;
    use crate::sync::watcher::{WatcherFailureKind, worker};
    use std::sync::{Arc, RwLock, mpsc};
    use std::time::{Duration, Instant};

    #[test]
    fn repo_watcher_bounded_shutdown_returns_without_drop_join() {
        let (command_tx, _command_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::channel();
        let join = std::thread::spawn(move || {
            let _ = release_rx.recv();
            Ok(())
        });
        let handle = RepoWatcherHandle {
            repo_id: uuid::Uuid::new_v4(),
            generation: 1,
            state: Arc::new(RwLock::new(worker::WorkerStateSlot::running(1))),
            command_tx: Some(command_tx),
            join: Some(join),
        };

        let started = Instant::now();
        let failure = handle
            .shutdown_bounded(Duration::from_millis(10))
            .expect_err("blocked worker must hit the deadline");
        assert_eq!(failure.kind, WatcherFailureKind::Coordination);
        assert!(started.elapsed() < Duration::from_secs(1));
        release_tx.send(()).expect("release detached join waiter");
    }
}
