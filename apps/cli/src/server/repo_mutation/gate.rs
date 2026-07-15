//! Repo/catalog lane registry, nesting guard, and permit lifecycle.

use super::MutationExecution;
use deve_core::models::RepoId;
use deve_core::protocol::ServerMessage;
use std::collections::HashMap;
use std::fmt;
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
        }
    }
}

impl std::error::Error for RepoMutationGateError {}

#[derive(Default)]
pub(crate) struct RepoMutationPublicationGate {
    locks: Mutex<HashMap<MutationLane, Weak<AsyncMutex<()>>>>,
}

impl RepoMutationPublicationGate {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn execute_repo<T, E>(
        &self,
        repo_id: RepoId,
        tx: &broadcast::Sender<ServerMessage>,
        operation: impl FnOnce() -> MutationExecution<T, E>,
    ) -> Result<MutationExecution<T, E>, RepoMutationGateError> {
        self.execute_lane(MutationLane::Repo(repo_id), tx, operation)
            .await
    }

    pub(crate) async fn execute_catalog<T, E>(
        &self,
        tx: &broadcast::Sender<ServerMessage>,
        operation: impl FnOnce() -> MutationExecution<T, E>,
    ) -> Result<MutationExecution<T, E>, RepoMutationGateError> {
        self.execute_lane(MutationLane::Catalog, tx, operation)
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
            .try_with(|held| held.catalog || held.conflicts(repo_lane))
            .unwrap_or(false)
        {
            return Err(RepoMutationGateError::NestedLane(repo_lane));
        }
        let catalog_lock = self.lane_lock(MutationLane::Catalog)?;
        let repo_lock = self.lane_lock(repo_lane)?;
        let _catalog_permit = catalog_lock.lock().await;
        let _repo_permit = repo_lock.lock().await;
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
        tx: &broadcast::Sender<ServerMessage>,
        operation: impl FnOnce() -> MutationExecution<T, E>,
    ) -> Result<MutationExecution<T, E>, RepoMutationGateError> {
        if HELD_MUTATION_LANES
            .try_with(|held| held.conflicts(lane))
            .unwrap_or(false)
        {
            return Err(RepoMutationGateError::NestedLane(lane));
        }
        let lock = self.lane_lock(lane)?;
        let _permit = lock.lock().await;
        let execution = HELD_MUTATION_LANES
            .scope(HeldMutationLanes::single(lane), async move { operation() })
            .await;
        if let Some(publication) = execution.publication() {
            publication.enqueue(tx);
        }
        Ok(execution)
    }

    fn lane_lock(&self, lane: MutationLane) -> Result<Arc<AsyncMutex<()>>, RepoMutationGateError> {
        let mut locks = self
            .locks
            .lock()
            .map_err(|_| RepoMutationGateError::RegistryPoisoned)?;
        locks.retain(|_, lock| lock.strong_count() > 0);
        if let Some(lock) = locks.get(&lane).and_then(Weak::upgrade) {
            return Ok(lock);
        }
        let lock = Arc::new(AsyncMutex::new(()));
        locks.insert(lane, Arc::downgrade(&lock));
        Ok(lock)
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
