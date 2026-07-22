//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 07_network#repo-control-wire-contract
//!
//! Host-owned repository lifecycle job owner.
//!
//! This module deliberately has no transport dependency. A caller may stop
//! waiting after admission, while the owned worker continues to convergence.

mod host;
mod model;
mod removal;
mod store;
mod worker;

pub(crate) use host::{RepoLifecycleHostExecutor, RepoLifecycleHostPublicationSink};
pub(crate) use model::{
    RepoLifecycleJobAccepted, RepoLifecycleJobError, RepoLifecycleJobExecutor,
    RepoLifecycleJobIntent, RepoLifecycleJobOperation, RepoLifecycleJobOutcome,
    RepoLifecycleJobPhase, RepoLifecycleJobStatus, RepoLifecyclePublicationSink,
    RepoLifecycleSettledPublication,
};
pub(crate) use removal::{
    RepoRemovalExecuteIntent, RepoRemovalIssuerBinding, RepoRemovalPrepareIntent,
    RepoRemovalPrepared,
};

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::{Notify, mpsc, oneshot};
use tokio::task::JoinHandle;
use uuid::Uuid;

const COMMAND_CAPACITY: usize = 32;

pub(crate) struct RepoLifecycleJobRuntime {
    accepting: AtomicBool,
    shutdown_started: AtomicBool,
    commands: mpsc::Sender<worker::Command>,
    worker: Mutex<Option<JoinHandle<Result<(), RepoLifecycleJobError>>>>,
    shutdown_result: Mutex<Option<Result<(), String>>>,
    shutdown_notify: Notify,
}

impl RepoLifecycleJobRuntime {
    pub(crate) fn start(
        ledger_dir: &Path,
        executor: Arc<dyn RepoLifecycleJobExecutor>,
        publication_sink: Arc<dyn RepoLifecyclePublicationSink>,
    ) -> Result<Arc<Self>, RepoLifecycleJobError> {
        let store = store::ReceiptStore::open(ledger_dir)?;
        let runtime_incarnation = Uuid::new_v4();
        let (commands, receiver) = mpsc::channel(COMMAND_CAPACITY);
        let worker = tokio::spawn(worker::run(
            store,
            executor,
            publication_sink,
            runtime_incarnation,
            receiver,
        ));
        Ok(Arc::new(Self {
            accepting: AtomicBool::new(true),
            shutdown_started: AtomicBool::new(false),
            commands,
            worker: Mutex::new(Some(worker)),
            shutdown_result: Mutex::new(None),
            shutdown_notify: Notify::new(),
        }))
    }

    pub(crate) async fn prepare_removal(
        &self,
        intent: RepoRemovalPrepareIntent,
    ) -> Result<RepoRemovalPrepared, RepoLifecycleJobError> {
        if !self.accepting.load(Ordering::Acquire) {
            return Err(RepoLifecycleJobError::AdmissionClosed);
        }
        let (reply, response) = oneshot::channel();
        self.commands
            .send(worker::Command::PrepareRemoval { intent, reply })
            .await
            .map_err(|_| self.closed_or_coordination())?;
        response.await.map_err(|_| self.closed_or_coordination())?
    }

    pub(crate) async fn execute_removal(
        &self,
        intent: RepoRemovalExecuteIntent,
    ) -> Result<RepoLifecycleJobAccepted, RepoLifecycleJobError> {
        if !self.accepting.load(Ordering::Acquire) {
            return Err(RepoLifecycleJobError::AdmissionClosed);
        }
        let (reply, response) = oneshot::channel();
        self.commands
            .send(worker::Command::ExecuteRemoval {
                intent,
                now_ms: None,
                reply,
            })
            .await
            .map_err(|_| self.closed_or_coordination())?;
        response.await.map_err(|_| self.closed_or_coordination())?
    }

    #[cfg(test)]
    async fn execute_removal_at_for_test(
        &self,
        intent: RepoRemovalExecuteIntent,
        now_ms: i64,
    ) -> Result<RepoLifecycleJobAccepted, RepoLifecycleJobError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .send(worker::Command::ExecuteRemoval {
                intent,
                now_ms: Some(now_ms),
                reply,
            })
            .await
            .map_err(|_| self.closed_or_coordination())?;
        response.await.map_err(|_| self.closed_or_coordination())?
    }

    pub(crate) async fn submit(
        &self,
        request_id: Uuid,
        intent: RepoLifecycleJobIntent,
    ) -> Result<RepoLifecycleJobAccepted, RepoLifecycleJobError> {
        if !self.accepting.load(Ordering::Acquire) {
            return Err(RepoLifecycleJobError::AdmissionClosed);
        }
        let (reply, response) = oneshot::channel();
        self.commands
            .send(worker::Command::Submit {
                request_id,
                intent,
                reply,
            })
            .await
            .map_err(|_| self.closed_or_coordination())?;
        response.await.map_err(|_| self.closed_or_coordination())?
    }

    pub(crate) async fn status(
        &self,
        request_id: Uuid,
    ) -> Result<RepoLifecycleJobStatus, RepoLifecycleJobError> {
        let (reply, response) = oneshot::channel();
        self.commands
            .send(worker::Command::Status { request_id, reply })
            .await
            .map_err(|_| self.closed_or_coordination())?;
        response.await.map_err(|_| self.closed_or_coordination())?
    }

    pub(crate) async fn shutdown(self: &Arc<Self>) -> Result<(), RepoLifecycleJobError> {
        self.accepting.store(false, Ordering::Release);
        if self
            .shutdown_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let runtime = self.clone();
            tokio::spawn(async move { runtime.drive_shutdown().await });
        }
        loop {
            let notified = self.shutdown_notify.notified();
            if let Some(result) = self
                .shutdown_result
                .lock()
                .map_err(|_| RepoLifecycleJobError::Coordination("shutdown state poisoned"))?
                .clone()
            {
                return result.map_err(RepoLifecycleJobError::Shutdown);
            }
            notified.await;
        }
    }

    async fn drive_shutdown(self: Arc<Self>) {
        let result = self
            .shutdown_worker()
            .await
            .map_err(|error| error.to_string());
        match self.shutdown_result.lock() {
            Ok(mut slot) => *slot = Some(result),
            Err(poisoned) => *poisoned.into_inner() = Some(Err("shutdown state poisoned".into())),
        }
        self.shutdown_notify.notify_waiters();
    }

    async fn shutdown_worker(&self) -> Result<(), RepoLifecycleJobError> {
        let (reply, response) = oneshot::channel();
        let command_result = match self
            .commands
            .send(worker::Command::Shutdown { reply })
            .await
        {
            Ok(()) => response.await.map_err(|_| {
                RepoLifecycleJobError::Coordination("lifecycle shutdown reply dropped")
            })?,
            Err(_) => Err(RepoLifecycleJobError::Coordination(
                "lifecycle worker stopped",
            )),
        };
        let worker = self
            .worker
            .lock()
            .map_err(|_| RepoLifecycleJobError::Coordination("lifecycle join lock poisoned"))?
            .take();
        let join_result = match worker {
            Some(worker) => worker
                .await
                .map_err(|_| RepoLifecycleJobError::Coordination("lifecycle worker join failed"))?,
            None => Ok(()),
        };
        match (command_result, join_result) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(primary), Ok(())) | (Ok(()), Err(primary)) => Err(primary),
            (Err(primary), Err(join)) => Err(RepoLifecycleJobError::Shutdown(format!(
                "{primary}; worker join also failed: {join}"
            ))),
        }
    }

    fn closed_or_coordination(&self) -> RepoLifecycleJobError {
        if self.accepting.load(Ordering::Acquire) {
            RepoLifecycleJobError::Coordination("lifecycle worker stopped")
        } else {
            RepoLifecycleJobError::AdmissionClosed
        }
    }

    #[cfg(test)]
    async fn abort_for_test(&self) {
        self.accepting.store(false, Ordering::Release);
        self.shutdown_started.store(true, Ordering::Release);
        let worker = self.worker.lock().expect("test lifecycle join lock").take();
        if let Some(worker) = worker {
            worker.abort();
            let _ = worker.await;
        }
    }
}

impl Drop for RepoLifecycleJobRuntime {
    fn drop(&mut self) {
        self.accepting.store(false, Ordering::Release);
        let Ok(worker) = self.worker.get_mut() else {
            return;
        };
        if let Some(worker) = worker.take() {
            tracing::error!(
                "RepoLifecycleJobRuntime dropped without explicit shutdown; aborting worker"
            );
            worker.abort();
        }
    }
}

#[cfg(test)]
mod tests;
