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
use std::sync::{Arc, Mutex, OnceLock};

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
            deferred_refresh: Mutex::new(DeferredRefresh::default()),
        }
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    pub(super) fn state(&self) -> RepoMountState {
        RepoMountState::decode(self.state.load(Ordering::Acquire))
    }

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
        loop {
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
                if let Some(refresh) = route.take() {
                    publisher(refresh);
                }
                return Ok(());
            }
        }
    }

    pub(super) fn restore_after_cancel(
        &self,
        state: RepoMountState,
        publisher: &WatcherRefreshCallback,
    ) -> Result<(), WatcherLifecycleError> {
        let mut route = self.deferred_refresh.lock().map_err(|_| {
            WatcherLifecycleError::Coordination(
                "watcher refresh route poisoned while cancelling lifecycle transition",
            )
        })?;
        self.state.store(state.encode(), Ordering::Release);
        if state == RepoMountState::Mounted {
            if let Some(refresh) = route.take() {
                publisher(refresh);
            }
        } else {
            route.clear();
        }
        Ok(())
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
        let Ok(mut route) = self.deferred_refresh.lock() else {
            self.fail(coordination_failure(
                "watcher refresh route poisoned while dispatching refresh",
            ));
            return;
        };
        match self.state() {
            RepoMountState::Mounted => publisher(refresh),
            RepoMountState::Starting | RepoMountState::Transitioning => route.push(refresh),
            RepoMountState::Failed | RepoMountState::Stopped => {}
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
        self.record_failure(failure);
        let _route = match self.deferred_refresh.lock() {
            Ok(route) => route,
            Err(poisoned) => poisoned.into_inner(),
        };
        if self.state() != RepoMountState::Stopped {
            self.state.store(MOUNT_FAILED, Ordering::Release);
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
        self.record_failure(failure);
        match self.deferred_refresh.lock() {
            Ok(mut route) => route.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
        self.state.store(MOUNT_FAILED, Ordering::Release);
    }

    fn record_failure(&self, failure: WatcherFailure) {
        let _ = self.failure.set(failure);
    }

    pub(super) fn recorded_failure(&self) -> Option<WatcherFailure> {
        self.failure.get().cloned()
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

    #[cfg(test)]
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    #[cfg(test)]
    pub(crate) fn state(&self) -> RepoMountState {
        self.state
    }

    pub(crate) fn failure(&self) -> Option<&WatcherFailure> {
        self.failure.as_ref()
    }
}
