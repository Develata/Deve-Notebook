//! Bounded, cancellable DiffProjection execution for WebSocket sessions.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 07_network#server-ws-runtime

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{
    MAX_WS_FRAME_BYTES, ScopeNonce, ServerError, ServerErrorCode, ServerMessage,
    server_binary_payload_size,
};
use deve_core::source_control::diff_projection::{
    DiffProjectionError, compute_diff_projection_cancellable,
};
use tokio::sync::{Notify, Semaphore};

use super::channel::DualChannel;

const MAX_DIFF_WORKERS: usize = 2;

pub(crate) struct DiffProjectionExecutor {
    permits: Arc<Semaphore>,
}

impl DiffProjectionExecutor {
    pub(crate) fn new() -> Self {
        let parallelism = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
            .clamp(1, MAX_DIFF_WORKERS);
        Self {
            permits: Arc::new(Semaphore::new(parallelism)),
        }
    }

    pub(crate) fn spawn(
        self: Arc<Self>,
        ticket: DiffJobTicket,
        base_content: String,
        target_content: String,
        response: DiffJobResponse,
        channel: DualChannel,
    ) {
        self.spawn_loaded(
            ticket,
            move || Ok((base_content, target_content, response)),
            channel,
        );
    }

    pub(crate) fn spawn_loaded<F>(
        self: Arc<Self>,
        ticket: DiffJobTicket,
        loader: F,
        channel: DualChannel,
    ) where
        F: FnOnce() -> Result<(String, String, DiffJobResponse), ServerError> + Send + 'static,
    {
        tokio::spawn(async move {
            let serial = tokio::select! {
                _ = ticket.cancellation.wait() => return,
                permit = ticket.serial.clone().acquire_owned() => match permit {
                    Ok(permit) => permit,
                    Err(_) => return,
                },
            };
            let permit = tokio::select! {
                _ = ticket.cancellation.wait() => return,
                permit = self.permits.clone().acquire_owned() => match permit {
                    Ok(permit) => permit,
                    Err(_) => return,
                },
            };
            let cancellation = ticket.cancellation.clone();
            let result = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                if cancellation.is_cancelled() {
                    return Err(DiffJobFailure::Cancelled);
                }
                let (base_content, target_content, response) =
                    loader().map_err(DiffJobFailure::Server)?;
                let projection =
                    compute_diff_projection_cancellable(base_content, target_content, &|| {
                        cancellation.is_cancelled()
                    })
                    .map_err(DiffJobFailure::Projection)?;
                Ok((projection, response))
            })
            .await;
            match result {
                Ok(Ok((projection, response))) => {
                    ticket
                        .publish_projection(&channel, projection, response)
                        .await
                }
                Ok(Err(DiffJobFailure::Cancelled))
                | Ok(Err(DiffJobFailure::Projection(DiffProjectionError::Cancelled))) => {}
                Ok(Err(DiffJobFailure::Projection(error))) => {
                    ticket.publish_error(&channel, error_payload(&error)).await
                }
                Ok(Err(DiffJobFailure::Server(error))) => {
                    ticket.publish_error(&channel, error).await
                }
                Err(_) => {
                    ticket
                        .publish_error(
                            &channel,
                            ServerError::with_detail(
                                ServerErrorCode::DiffComputeFailed,
                                "diff worker failed",
                            ),
                        )
                        .await
                }
            }
            drop(serial);
        });
    }

    pub(crate) async fn run_bounded<T, F>(&self, job: F) -> Result<T, ServerError>
    where
        T: Send + 'static,
        F: FnOnce() -> Result<T, ServerError> + Send + 'static,
    {
        let permit = self.permits.clone().acquire_owned().await.map_err(|_| {
            ServerError::with_detail(
                ServerErrorCode::DiffComputeFailed,
                "diff worker pool is closed",
            )
        })?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            job()
        })
        .await
        .map_err(|_| {
            ServerError::with_detail(ServerErrorCode::DiffComputeFailed, "diff worker failed")
        })?
    }
}

enum DiffJobFailure {
    Cancelled,
    Projection(DiffProjectionError),
    Server(ServerError),
}

pub(crate) enum DiffJobResponse {
    Draft,
    Document {
        doc_id: Option<deve_core::models::DocId>,
        path: String,
    },
    Merge {
        doc_id: deve_core::models::DocId,
        path: String,
        result_content: String,
        actions: Vec<deve_core::protocol::MergeConflictAction>,
        conflicts: Vec<deve_core::protocol::ConflictHunk>,
    },
}

#[derive(Clone)]
pub(crate) struct DiffJobGate {
    inner: Arc<Mutex<GateState>>,
    serial: Arc<Semaphore>,
}

struct GateState {
    generation: u64,
    latest_draft_revision: u64,
    active: Option<ActiveJob>,
}

struct ActiveJob {
    request_id: String,
    revision: u64,
    repo_id: RepoId,
    branch: Option<PeerId>,
    scope_nonce: ScopeNonce,
    cancellation: Arc<JobCancellation>,
}

struct JobCancellation {
    cancelled: AtomicBool,
    notify: Notify,
}

pub(crate) struct DiffJobTicket {
    gate: DiffJobGate,
    generation: u64,
    request_id: String,
    revision: u64,
    repo_id: RepoId,
    branch: Option<PeerId>,
    scope_nonce: ScopeNonce,
    cancellation: Arc<JobCancellation>,
    serial: Arc<Semaphore>,
}

impl DiffJobGate {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(GateState {
                generation: 0,
                latest_draft_revision: 0,
                active: None,
            })),
            serial: Arc::new(Semaphore::new(1)),
        }
    }

    pub(crate) fn begin_draft(
        &self,
        request_id: String,
        revision: u64,
        repo_id: RepoId,
        branch: Option<PeerId>,
        scope_nonce: ScopeNonce,
    ) -> Option<DiffJobTicket> {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if revision <= state.latest_draft_revision {
            return None;
        }
        state.latest_draft_revision = revision;
        Some(self.replace_active(
            &mut state,
            request_id,
            revision,
            repo_id,
            branch,
            scope_nonce,
        ))
    }

    pub(crate) fn begin_fixed(
        &self,
        request_id: String,
        repo_id: RepoId,
        branch: Option<PeerId>,
        scope_nonce: ScopeNonce,
    ) -> DiffJobTicket {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        self.replace_active(&mut state, request_id, 0, repo_id, branch, scope_nonce)
    }

    fn replace_active(
        &self,
        state: &mut GateState,
        request_id: String,
        revision: u64,
        repo_id: RepoId,
        branch: Option<PeerId>,
        scope_nonce: ScopeNonce,
    ) -> DiffJobTicket {
        if let Some(active) = state.active.take() {
            active.cancellation.cancel();
        }
        state.generation = state.generation.wrapping_add(1);
        let generation = state.generation;
        let cancellation = Arc::new(JobCancellation::new());
        state.active = Some(ActiveJob {
            request_id: request_id.clone(),
            revision,
            repo_id,
            branch: branch.clone(),
            scope_nonce,
            cancellation: cancellation.clone(),
        });
        DiffJobTicket {
            gate: self.clone(),
            generation,
            request_id,
            revision,
            repo_id,
            branch,
            scope_nonce,
            cancellation,
            serial: self.serial.clone(),
        }
    }

    pub(crate) fn cancel(&self) {
        let mut state = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(active) = state.active.take() {
            active.cancellation.cancel();
        }
        state.latest_draft_revision = 0;
        state.generation = state.generation.wrapping_add(1);
    }
}

impl JobCancellation {
    fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
            notify: Notify::new(),
        }
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    async fn wait(&self) {
        loop {
            let notified = self.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

impl DiffJobTicket {
    async fn publish_projection(
        self,
        channel: &DualChannel,
        projection: deve_core::source_control::diff_projection::DiffProjection,
        response: DiffJobResponse,
    ) {
        let message = match response {
            DiffJobResponse::Draft => ServerMessage::DiffProjectionResult {
                request_id: self.request_id.clone(),
                revision: self.revision,
                repo_id: self.repo_id,
                branch: self.branch.clone(),
                scope_nonce: self.scope_nonce,
                projection: Arc::new(projection),
            },
            DiffJobResponse::Document { doc_id, path } => ServerMessage::DocDiff {
                request_id: Some(self.request_id.clone()),
                repo_id: Some(self.repo_id),
                branch: self.branch.clone(),
                scope_nonce: Some(self.scope_nonce.get()),
                doc_id,
                path,
                projection: Arc::new(projection),
            },
            DiffJobResponse::Merge {
                doc_id,
                path,
                result_content,
                actions,
                conflicts,
            } => ServerMessage::MergeConflict {
                repo_id: Some(self.repo_id),
                branch: self.branch.clone(),
                scope_nonce: Some(self.scope_nonce.get()),
                doc_id,
                path,
                projection: Arc::new(projection),
                result_content,
                actions,
                conflicts,
            },
        };
        match server_binary_payload_size(&message) {
            Ok(size) if size <= MAX_WS_FRAME_BYTES => self.publish(channel, message).await,
            Ok(size) => {
                self.publish_error(
                    channel,
                    ServerError::with_detail(
                        ServerErrorCode::DiffResourceLimit,
                        format!("encoded_bytes={size}; limit={MAX_WS_FRAME_BYTES}"),
                    ),
                )
                .await
            }
            Err(_) => {
                self.publish_error(
                    channel,
                    ServerError::new(ServerErrorCode::DiffComputeFailed),
                )
                .await
            }
        }
    }

    async fn publish_error(self, channel: &DualChannel, error: ServerError) {
        let message = ServerMessage::DiffProjectionError {
            request_id: self.request_id.clone(),
            revision: self.revision,
            repo_id: self.repo_id,
            branch: self.branch.clone(),
            scope_nonce: self.scope_nonce,
            error,
        };
        self.publish(channel, message).await;
    }

    async fn publish(self, channel: &DualChannel, message: ServerMessage) {
        if !self.is_current() || self.cancellation.is_cancelled() {
            return;
        }
        let sender = channel.diff_unicast_sender();
        let permit = tokio::select! {
            _ = self.cancellation.wait() => return,
            permit = sender.reserve_owned() => match permit {
                Ok(permit) => permit,
                Err(_) => return,
            },
        };
        let mut state = self
            .gate
            .inner
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let current = state.generation == self.generation
            && state.active.as_ref().is_some_and(|active| {
                active.request_id == self.request_id
                    && active.revision == self.revision
                    && active.repo_id == self.repo_id
                    && active.branch == self.branch
                    && active.scope_nonce == self.scope_nonce
            });
        if current && !self.cancellation.is_cancelled() {
            permit.send(message);
            state.active = None;
        }
    }

    fn is_current(&self) -> bool {
        let state = self
            .gate
            .inner
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        state.generation == self.generation
            && state.active.as_ref().is_some_and(|active| {
                active.request_id == self.request_id
                    && active.revision == self.revision
                    && active.repo_id == self.repo_id
                    && active.branch == self.branch
                    && active.scope_nonce == self.scope_nonce
            })
    }
}

fn error_payload(error: &DiffProjectionError) -> ServerError {
    let code = if error.is_resource_limit() {
        ServerErrorCode::DiffResourceLimit
    } else {
        ServerErrorCode::DiffComputeFailed
    };
    ServerError::with_detail(code, error.to_string())
}

#[cfg(test)]
#[path = "diff_projection/tests.rs"]
mod tests;
