//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#repo-health-and-repair
//!
//! Exact generation slot and refresh linearization state.

use super::error::WatcherLifecycleError;
use super::refresh_route::DeferredRefresh;
use deve_core::models::RepoId;
use deve_core::sync::watcher::{
    WatcherFailure, WatcherFailureKind, WatcherFailurePhase, WatcherRefresh, WatcherRefreshCallback,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock, TryLockError};

const MOUNT_STARTING: u8 = 0;
const MOUNT_MOUNTED: u8 = 1;
const MOUNT_TRANSITIONING: u8 = 2;
const MOUNT_FAILED: u8 = 3;
const MOUNT_STOPPED: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RepoMountState {
    Starting,
    Mounted,
    Transitioning,
    Failed,
    Stopped,
}

impl RepoMountState {
    pub(super) fn encode(self) -> u8 {
        match self {
            Self::Starting => MOUNT_STARTING,
            Self::Mounted => MOUNT_MOUNTED,
            Self::Transitioning => MOUNT_TRANSITIONING,
            Self::Failed => MOUNT_FAILED,
            Self::Stopped => MOUNT_STOPPED,
        }
    }

    fn decode(value: u8) -> Self {
        match value {
            MOUNT_STARTING => Self::Starting,
            MOUNT_MOUNTED => Self::Mounted,
            MOUNT_TRANSITIONING => Self::Transitioning,
            MOUNT_FAILED => Self::Failed,
            MOUNT_STOPPED => Self::Stopped,
            _ => Self::Failed,
        }
    }
}

pub(super) struct MountSlot {
    pub(super) repo_id: RepoId,
    pub(super) generation: u64,
    pub(super) state: AtomicU8,
    failure: OnceLock<WatcherFailure>,
    failure_cleanup: Mutex<Vec<String>>,
    deferred_refresh: Mutex<DeferredRefresh>,
}

impl MountSlot {
    pub(super) fn starting(repo_id: RepoId, generation: u64) -> Self {
        Self::with_state(repo_id, generation, MOUNT_STARTING)
    }

    pub(super) fn transitioning(repo_id: RepoId, generation: u64) -> Self {
        Self::with_state(repo_id, generation, MOUNT_TRANSITIONING)
    }

    #[cfg(test)]
    pub(super) fn mounted(repo_id: RepoId, generation: u64) -> Self {
        Self::with_state(repo_id, generation, MOUNT_MOUNTED)
    }

    fn with_state(repo_id: RepoId, generation: u64, state: u8) -> Self {
        Self {
            repo_id,
            generation,
            state: AtomicU8::new(state),
            failure: OnceLock::new(),
            failure_cleanup: Mutex::new(Vec::new()),
            deferred_refresh: Mutex::new(DeferredRefresh::default()),
        }
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn state(&self) -> RepoMountState {
        RepoMountState::decode(self.state.load(Ordering::Acquire))
    }

    #[allow(dead_code)] // R4 ownership-aware remove consumes this transition.
    pub(super) fn begin_lifecycle_transition(&self) -> Result<RepoMountState, SlotTransitionError> {
        let _route = self
            .deferred_refresh
            .lock()
            .map_err(|_| SlotTransitionError::Coordination)?;
        loop {
            let current = self.state.load(Ordering::Acquire);
            if matches!(current, MOUNT_STARTING | MOUNT_TRANSITIONING) {
                return Err(SlotTransitionError::Busy(RepoMountState::decode(current)));
            }
            if self
                .state
                .compare_exchange_weak(
                    current,
                    MOUNT_TRANSITIONING,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return Ok(RepoMountState::decode(current));
            }
        }
    }

    pub(super) fn mark_mounted(
        &self,
        publisher: &WatcherRefreshCallback,
    ) -> Result<(), WatcherFailure> {
        let mut route = self.deferred_refresh.lock().map_err(|_| {
            coordination_failure("watcher refresh route poisoned during mount handoff")
        })?;
        let refresh = loop {
            let current = self.state.load(Ordering::Acquire);
            if !matches!(current, MOUNT_STARTING | MOUNT_TRANSITIONING) {
                return Err(self.recorded_failure().unwrap_or_else(|| {
                    coordination_failure(format!(
                        "watcher mount handoff found unexpected {:?} state",
                        RepoMountState::decode(current)
                    ))
                }));
            }
            if self
                .state
                .compare_exchange_weak(current, MOUNT_MOUNTED, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break route.take();
            }
        };
        drop(route);
        if let Some(refresh) = refresh {
            publisher(refresh);
        }
        Ok(())
    }

    pub(super) fn restore_after_cancel(
        &self,
        state: RepoMountState,
    ) -> Result<Option<WatcherRefresh>, WatcherLifecycleError> {
        let mut route = self.deferred_refresh.lock().map_err(|_| {
            WatcherLifecycleError::Coordination(
                "watcher refresh route poisoned while cancelling lifecycle transition",
            )
        })?;
        if self.failure.get().is_some() {
            self.state.store(MOUNT_FAILED, Ordering::Release);
            route.clear();
            return Ok(None);
        }
        match self.state.compare_exchange(
            MOUNT_TRANSITIONING,
            state.encode(),
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) if state == RepoMountState::Mounted => Ok(route.take()),
            Ok(_) => {
                route.clear();
                Ok(None)
            }
            Err(MOUNT_FAILED | MOUNT_STOPPED) => {
                route.clear();
                Ok(None)
            }
            Err(_) => Err(WatcherLifecycleError::Coordination(
                "watcher lifecycle cancellation observed an invalid previous slot state",
            )),
        }
    }

    pub(super) fn route_refresh(
        &self,
        refresh: WatcherRefresh,
        publisher: &WatcherRefreshCallback,
    ) {
        if refresh.repo_id() != self.repo_id {
            self.fail(coordination_failure(format!(
                "watcher refresh repo mismatch: slot={}, refresh={}",
                self.repo_id,
                refresh.repo_id()
            )));
            return;
        }
        let publication = {
            let Ok(mut route) = self.deferred_refresh.lock() else {
                self.fail(coordination_failure(
                    "watcher refresh route poisoned while dispatching refresh",
                ));
                return;
            };
            match self.state() {
                RepoMountState::Mounted => Some(refresh),
                RepoMountState::Starting | RepoMountState::Transitioning => {
                    route.push(refresh);
                    None
                }
                RepoMountState::Failed | RepoMountState::Stopped => None,
            }
        };
        if let Some(refresh) = publication {
            publisher(refresh);
        }
    }

    pub(super) fn merge_deferred(&self, refresh: Option<WatcherRefresh>) -> Result<(), ()> {
        let mut route = self.deferred_refresh.lock().map_err(|_| ())?;
        if let Some(refresh) = refresh {
            route.push(refresh);
        }
        Ok(())
    }

    pub(super) fn take_deferred(&self) -> Result<Option<WatcherRefresh>, ()> {
        self.deferred_refresh
            .lock()
            .map(|mut route| route.take())
            .map_err(|_| ())
    }

    pub(super) fn drop_deferred(&self) {
        match self.deferred_refresh.lock() {
            Ok(mut route) => route.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }

    pub(super) fn fail(&self, failure: WatcherFailure) {
        let _ = self.failure.set(failure);
        loop {
            let current = self.state.load(Ordering::Acquire);
            if matches!(current, MOUNT_FAILED | MOUNT_STOPPED) {
                break;
            }
            if self
                .state
                .compare_exchange_weak(current, MOUNT_FAILED, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                break;
            }
        }
        match self.deferred_refresh.try_lock() {
            Ok(mut route) => route.clear(),
            Err(TryLockError::Poisoned(poisoned)) => poisoned.into_inner().clear(),
            Err(TryLockError::WouldBlock) => {}
        }
    }

    pub(super) fn begin_shutdown(&self) {
        let _route = match self.deferred_refresh.lock() {
            Ok(route) => route,
            Err(poisoned) => poisoned.into_inner(),
        };
        if self.state() != RepoMountState::Stopped {
            self.state.store(MOUNT_TRANSITIONING, Ordering::Release);
        }
    }

    pub(super) fn mark_stopped_preserving_refresh(&self) {
        let _route = match self.deferred_refresh.lock() {
            Ok(route) => route,
            Err(poisoned) => poisoned.into_inner(),
        };
        self.state.store(MOUNT_STOPPED, Ordering::Release);
    }

    pub(super) fn mark_failed_and_drop(&self, failure: WatcherFailure) {
        self.merge_failure(failure);
        self.state.store(MOUNT_FAILED, Ordering::Release);
        match self.deferred_refresh.lock() {
            Ok(mut route) => route.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }

    fn merge_failure(&self, failure: WatcherFailure) {
        if self.failure.set(failure.clone()).is_ok() {
            return;
        }
        let Some(primary) = self.failure.get() else {
            return;
        };
        let distinct_failure = (primary.phase != failure.phase
            || primary.kind != failure.kind
            || primary.primary != failure.primary)
            .then(|| failure.to_string());
        let mut additions = failure.cleanup;
        if let Some(distinct_failure) = distinct_failure {
            additions.insert(0, distinct_failure);
        }
        if additions.is_empty() {
            return;
        }
        match self.failure_cleanup.lock() {
            Ok(mut cleanup) => cleanup.extend(additions),
            Err(poisoned) => poisoned.into_inner().extend(additions),
        }
    }

    pub(super) fn recorded_failure(&self) -> Option<WatcherFailure> {
        let mut failure = self.failure.get()?.clone();
        match self.failure_cleanup.lock() {
            Ok(cleanup) => failure.cleanup.extend(cleanup.iter().cloned()),
            Err(poisoned) => failure
                .cleanup
                .extend(poisoned.into_inner().iter().cloned()),
        }
        Some(failure)
    }

    pub(super) fn snapshot(&self) -> WatcherMountSnapshot {
        WatcherMountSnapshot {
            repo_id: self.repo_id,
            generation: self.generation,
            state: self.state(),
            failure: self.recorded_failure(),
        }
    }
}

fn coordination_failure(primary: impl Into<String>) -> WatcherFailure {
    WatcherFailure::new(
        WatcherFailurePhase::Worker,
        WatcherFailureKind::Coordination,
        primary,
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // R4 ownership-aware remove consumes this typed failure.
pub(super) enum SlotTransitionError {
    Busy(RepoMountState),
    Coordination,
}

pub(super) type RuntimeSlots = Arc<Mutex<HashMap<RepoId, Arc<MountSlot>>>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WatcherMountSnapshot {
    pub(super) repo_id: RepoId,
    pub(super) generation: u64,
    pub(super) state: RepoMountState,
    pub(super) failure: Option<WatcherFailure>,
}

impl WatcherMountSnapshot {
    pub(crate) fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn state(&self) -> RepoMountState {
        self.state
    }

    pub(crate) fn failure(&self) -> Option<&WatcherFailure> {
        self.failure.as_ref()
    }
}
