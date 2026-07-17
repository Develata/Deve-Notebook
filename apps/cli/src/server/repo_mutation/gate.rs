//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Repo/catalog lane registry, nesting guard, and permit lifecycle.

use super::MutationExecution;
use crate::server::runtime::watcher_runtime::{
    MountAdmissionError, MountAdmissionToken, MountContinuationToken, WatcherRuntimeView,
};
use deve_core::models::RepoId;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::{Mutex as AsyncMutex, broadcast};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum MutationLane {
    Repo(RepoId),
    Catalog,
}

tokio::task_local! {
    static HELD_MUTATION_LANES: HeldMutationLanes;
}

#[derive(Clone, Copy, Debug, Default)]
struct HeldMutationLanes {
    catalog: bool,
    repo: Option<RepoId>,
}

impl HeldMutationLanes {
    fn single(lane: MutationLane) -> Self {
        match lane {
            MutationLane::Repo(repo_id) => Self {
                catalog: false,
                repo: Some(repo_id),
            },
            MutationLane::Catalog => Self {
                catalog: true,
                repo: None,
            },
        }
    }

    fn conflicts(self, lane: MutationLane) -> bool {
        match lane {
            MutationLane::Repo(repo_id) => self.repo == Some(repo_id),
            // A catalog permit may only be acquired before a repo permit.
            // Reject Repo -> Catalog nesting so it cannot deadlock against the
            // supported Catalog -> Repo order in `execute_catalog_repo`.
            MutationLane::Catalog => self.catalog || self.repo.is_some(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepoMutationGateError {
    RegistryPoisoned,
    NestedLane(MutationLane),
    WorkspaceIngestionUnavailable,
    ContinuationSuperseded,
    BlockingExecutionFailed,
}

impl fmt::Display for RepoMutationGateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryPoisoned => write!(f, "repo mutation gate registry is poisoned"),
            Self::NestedLane(MutationLane::Repo(repo_id)) => {
                write!(f, "nested repo mutation permit for {repo_id}")
            }
            Self::NestedLane(MutationLane::Catalog) => {
                write!(f, "nested repository catalog mutation permit")
            }
            Self::WorkspaceIngestionUnavailable => {
                write!(f, "workspace ingestion is unavailable")
            }
            Self::ContinuationSuperseded => {
                write!(f, "repository mutation continuation was superseded")
            }
            Self::BlockingExecutionFailed => {
                write!(f, "mounted repository blocking execution failed")
            }
        }
    }
}

impl std::error::Error for RepoMutationGateError {}

pub(crate) struct RepoMutationPublicationGate {
    locks: Mutex<HashMap<MutationLane, Weak<MutationLaneState>>>,
    watcher_runtime: WatcherRuntimeView,
}

struct MutationLaneState {
    permit: Arc<AsyncMutex<()>>,
    revision: AtomicU64,
}

impl MutationLaneState {
    fn new() -> Self {
        Self {
            permit: Arc::new(AsyncMutex::new(())),
            revision: AtomicU64::new(0),
        }
    }

    fn begin(&self) -> u64 {
        self.revision.fetch_add(1, Ordering::AcqRel) + 1
    }

    fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
}

/// Stable, read-only admission captured after scope and projection-health
/// checks but before lock-free preflight work. The exact slot and generation
/// are revalidated only after the repository mutation permit is acquired.
pub(crate) struct MountedRepoAdmission {
    token: MountAdmissionToken,
}

/// Completion authority for one operation that already crossed the Mounted
/// cut. It may finish cleanup after a repo-local watcher failure, but never
/// across lifecycle transition or slot replacement.
pub(crate) struct MountedRepoContinuation {
    token: MountContinuationToken,
    lane: Arc<MutationLaneState>,
    revision: u64,
}

impl MountedRepoAdmission {
    fn repo_id(&self) -> RepoId {
        self.token.repo_id()
    }
}

impl RepoMutationPublicationGate {
    pub(crate) fn new(watcher_runtime: WatcherRuntimeView) -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
            watcher_runtime,
        }
    }

    #[cfg(not(test))]
    pub(crate) fn uses_watcher_runtime(&self, watcher_runtime: &WatcherRuntimeView) -> bool {
        self.watcher_runtime.same_runtime(watcher_runtime)
    }

    pub(crate) async fn execute_mounted_repo<T, E>(
        &self,
        repo_id: RepoId,
        tx: &broadcast::Sender<ServerMessage>,
        operation: impl FnOnce() -> MutationExecution<T, E>,
    ) -> Result<MutationExecution<T, E>, RepoMutationGateError> {
        let admission = self.admit_mounted_repo(repo_id)?;
        self.execute_admitted_mounted_repo(admission, tx, operation)
            .await
    }

    pub(crate) fn admit_mounted_repo(
        &self,
        repo_id: RepoId,
    ) -> Result<MountedRepoAdmission, RepoMutationGateError> {
        self.watcher_runtime
            .admit(repo_id)
            .map(|token| MountedRepoAdmission { token })
            .map_err(RepoMutationGateError::from)
    }

    pub(crate) async fn execute_admitted_mounted_repo<T, E>(
        &self,
        admission: MountedRepoAdmission,
        tx: &broadcast::Sender<ServerMessage>,
        operation: impl FnOnce() -> MutationExecution<T, E>,
    ) -> Result<MutationExecution<T, E>, RepoMutationGateError> {
        let repo_id = admission.repo_id();
        self.execute_lane(
            MutationLane::Repo(repo_id),
            Some(admission.token),
            tx,
            operation,
        )
        .await
    }

    pub(crate) async fn execute_mounted_repo_unpublished<T, E>(
        &self,
        repo_id: RepoId,
        operation: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<T, E>, RepoMutationGateError> {
        let admission = self.admit_mounted_repo(repo_id)?;
        self.execute_admitted_mounted_repo_unpublished(admission, operation)
            .await
    }

    pub(crate) async fn execute_admitted_mounted_repo_unpublished<T, E>(
        &self,
        admission: MountedRepoAdmission,
        operation: impl FnOnce() -> Result<T, E>,
    ) -> Result<Result<T, E>, RepoMutationGateError> {
        let lane = MutationLane::Repo(admission.repo_id());
        if HELD_MUTATION_LANES
            .try_with(|held| held.conflicts(lane))
            .unwrap_or(false)
        {
            return Err(RepoMutationGateError::NestedLane(lane));
        }
        let lane_state = self.lane_state(lane)?;
        let _permit = lane_state.permit.lock().await;
        admission
            .token
            .revalidate()
            .map_err(RepoMutationGateError::from)?;
        lane_state.begin();
        let outcome = HELD_MUTATION_LANES
            .scope(HeldMutationLanes::single(lane), async move { operation() })
            .await;
        Ok(outcome)
    }

    /// Runs the local apply on Tokio's blocking pool and returns completion
    /// authority for a scan/finalize continuation. The blocking task owns the
    /// permit, so caller cancellation cannot release serialization early.
    pub(crate) async fn execute_admitted_mounted_repo_unpublished_blocking_with_continuation<T, E>(
        &self,
        admission: MountedRepoAdmission,
        operation: impl FnOnce() -> Result<T, E> + Send + 'static,
    ) -> Result<(Result<T, E>, MountedRepoContinuation), RepoMutationGateError>
    where
        T: Send + 'static,
        E: Send + 'static,
    {
        let lane = MutationLane::Repo(admission.repo_id());
        if HELD_MUTATION_LANES
            .try_with(|held| held.conflicts(lane))
            .unwrap_or(false)
        {
            return Err(RepoMutationGateError::NestedLane(lane));
        }
        let lane_state = self.lane_state(lane)?;
        let permit = lane_state.permit.clone().lock_owned().await;
        admission
            .token
            .revalidate()
            .map_err(RepoMutationGateError::from)?;
        let revision = lane_state.begin();
        let continuation = MountedRepoContinuation {
            token: admission.token.continuation(),
            lane: lane_state.clone(),
            revision,
        };
        let outcome = tokio::task::spawn_blocking(move || {
            let _lane_state = lane_state;
            let _permit = permit;
            HELD_MUTATION_LANES.sync_scope(HeldMutationLanes::single(lane), operation)
        })
        .await
        .map_err(|error| {
            tracing::error!(%error, "mounted repository blocking execution failed");
            RepoMutationGateError::BlockingExecutionFailed
        })?;
        Ok((outcome, continuation))
    }

    pub(crate) async fn execute_mounted_repo_continuation_unpublished_blocking<T, E>(
        &self,
        continuation: MountedRepoContinuation,
        operation: impl FnOnce() -> Result<T, E> + Send + 'static,
    ) -> Result<Result<T, E>, RepoMutationGateError>
    where
        T: Send + 'static,
        E: Send + 'static,
    {
        let lane = MutationLane::Repo(continuation.token.repo_id());
        if HELD_MUTATION_LANES
            .try_with(|held| held.conflicts(lane))
            .unwrap_or(false)
        {
            return Err(RepoMutationGateError::NestedLane(lane));
        }
        let permit = continuation.lane.permit.clone().lock_owned().await;
        continuation
            .token
            .revalidate()
            .map_err(RepoMutationGateError::from)?;
        if continuation.lane.revision() != continuation.revision {
            return Err(RepoMutationGateError::ContinuationSuperseded);
        }
        continuation.lane.begin();
        let lane_state = continuation.lane;
        tokio::task::spawn_blocking(move || {
            let _lane_state = lane_state;
            let _permit = permit;
            HELD_MUTATION_LANES.sync_scope(HeldMutationLanes::single(lane), operation)
        })
        .await
        .map_err(|error| {
            tracing::error!(%error, "mounted repository continuation failed");
            RepoMutationGateError::BlockingExecutionFailed
        })
    }

    pub(crate) async fn execute_catalog<T, E>(
        &self,
        tx: &broadcast::Sender<ServerMessage>,
        operation: impl FnOnce() -> MutationExecution<T, E>,
    ) -> Result<MutationExecution<T, E>, RepoMutationGateError> {
        self.execute_lane(MutationLane::Catalog, None, tx, operation)
            .await
    }

    /// Catalog identity mutations for an existing repo acquire the global
    /// catalog lane first and then the repo lane. This prevents create/name
    /// races while preserving a single lock order against document writers.
    pub(crate) async fn execute_catalog_repo<T, E>(
        &self,
        repo_id: RepoId,
        tx: &broadcast::Sender<ServerMessage>,
        operation: impl FnOnce() -> MutationExecution<T, E>,
    ) -> Result<MutationExecution<T, E>, RepoMutationGateError> {
        let repo_lane = MutationLane::Repo(repo_id);
        if HELD_MUTATION_LANES
            .try_with(|held| held.catalog || held.repo.is_some())
            .unwrap_or(false)
        {
            return Err(RepoMutationGateError::NestedLane(repo_lane));
        }
        let catalog_lane = self.lane_state(MutationLane::Catalog)?;
        let repo_lane_state = self.lane_state(repo_lane)?;
        let _catalog_permit = catalog_lane.permit.lock().await;
        let _repo_permit = repo_lane_state.permit.lock().await;
        catalog_lane.begin();
        repo_lane_state.begin();
        let execution = HELD_MUTATION_LANES
            .scope(
                HeldMutationLanes {
                    catalog: true,
                    repo: Some(repo_id),
                },
                async move { operation() },
            )
            .await;
        if let Some(publication) = execution.publication() {
            publication.enqueue(tx);
        }
        Ok(execution)
    }

    async fn execute_lane<T, E>(
        &self,
        lane: MutationLane,
        admission: Option<MountAdmissionToken>,
        tx: &broadcast::Sender<ServerMessage>,
        operation: impl FnOnce() -> MutationExecution<T, E>,
    ) -> Result<MutationExecution<T, E>, RepoMutationGateError> {
        if HELD_MUTATION_LANES
            .try_with(|held| held.conflicts(lane))
            .unwrap_or(false)
        {
            return Err(RepoMutationGateError::NestedLane(lane));
        }
        let lane_state = self.lane_state(lane)?;
        let _permit = lane_state.permit.lock().await;
        if let Some(admission) = admission {
            debug_assert_eq!(Some(admission.repo_id()), lane.repo_id());
            admission
                .revalidate()
                .map_err(RepoMutationGateError::from)?;
        }
        lane_state.begin();
        let execution = HELD_MUTATION_LANES
            .scope(HeldMutationLanes::single(lane), async move { operation() })
            .await;
        if let Some(publication) = execution.publication() {
            publication.enqueue(tx);
        }
        Ok(execution)
    }

    fn lane_state(
        &self,
        lane: MutationLane,
    ) -> Result<Arc<MutationLaneState>, RepoMutationGateError> {
        let mut locks = self
            .locks
            .lock()
            .map_err(|_| RepoMutationGateError::RegistryPoisoned)?;
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = locks.get(&lane).and_then(Weak::upgrade) {
            return Ok(lock);
        }
        let lane_state = Arc::new(MutationLaneState::new());
        locks.insert(lane, Arc::downgrade(&lane_state));
        Ok(lane_state)
    }

    #[cfg(test)]
    pub(super) fn live_registry_len(&self) -> Result<usize, RepoMutationGateError> {
        let mut locks = self
            .locks
            .lock()
            .map_err(|_| RepoMutationGateError::RegistryPoisoned)?;
        locks.retain(|_, lock| lock.strong_count() > 0);
        Ok(locks.len())
    }
}

impl MutationLane {
    fn repo_id(self) -> Option<RepoId> {
        match self {
            Self::Repo(repo_id) => Some(repo_id),
            Self::Catalog => None,
        }
    }
}

impl From<MountAdmissionError> for RepoMutationGateError {
    fn from(_: MountAdmissionError) -> Self {
        Self::WorkspaceIngestionUnavailable
    }
}

impl RepoMutationGateError {
    pub(crate) fn server_error(&self) -> ServerError {
        match self {
            Self::WorkspaceIngestionUnavailable => ServerError::workspace_ingestion_unavailable(),
            _ => ServerError::with_detail(ServerErrorCode::StoragePersistFailed, self.to_string()),
        }
    }
}
