//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#repo-health-and-repair
//!   - 07_network#server-ws-runtime
//!
//! Unique server owner of watcher handles, slots and generations.

use super::error::{
    WatcherLifecycleError, WatcherSupervisorShutdownError, WatcherSupervisorStartError,
};
use super::slot::{RepoMountState, RuntimeSlots};
use super::view::WatcherRuntimeView;
use deve_core::models::RepoId;
use deve_core::sync::watcher::{
    RepoWatcherHandle, RepoWatcherStart, WatcherFailure, WatcherFailureKind, WatcherFailurePhase,
    WatcherRefreshCallback,
};
use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const BOOTSTRAP_BUILDING: u8 = 0;
const BOOTSTRAP_RUNNING: u8 = 1;
const BOOTSTRAP_FAILED: u8 = 2;

pub(crate) struct WatcherSupervisor {
    pub(super) slots: RuntimeSlots,
    pub(super) owned: Mutex<OwnedHandles>,
    pub(super) publisher: WatcherRefreshCallback,
    #[cfg(test)]
    pub(super) fail_next_start: AtomicBool,
    #[cfg(test)]
    pub(super) fail_next_shutdown_after_cleanup: AtomicBool,
}

pub(super) struct OwnedHandles {
    pub(super) handles: HashMap<RepoId, OwnedHandle>,
    pub(super) start_order: Vec<HandleKey>,
    pub(super) shutting_down: bool,
}

pub(super) struct OwnedHandle {
    pub(super) generation: u64,
    pub(super) handle: RepoWatcherHandle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct HandleKey {
    pub(super) repo_id: RepoId,
    pub(super) generation: u64,
}

pub(super) struct BootstrapTracker {
    state: AtomicU8,
    failure: OnceLock<(RepoId, WatcherFailure)>,
}

impl BootstrapTracker {
    fn new() -> Self {
        Self {
            state: AtomicU8::new(BOOTSTRAP_BUILDING),
            failure: OnceLock::new(),
        }
    }

    pub(super) fn fail(&self, repo_id: RepoId, failure: WatcherFailure) {
        let _ = self.failure.set((repo_id, failure));
        let _ = self.state.compare_exchange(
            BOOTSTRAP_BUILDING,
            BOOTSTRAP_FAILED,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    fn seal(&self) -> bool {
        self.state
            .compare_exchange(
                BOOTSTRAP_BUILDING,
                BOOTSTRAP_RUNNING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }
}

impl WatcherSupervisor {
    pub(crate) fn start_all(
        starts: Vec<RepoWatcherStart>,
        publisher: WatcherRefreshCallback,
    ) -> Result<Self, WatcherSupervisorStartError> {
        let mut repo_ids = HashSet::with_capacity(starts.len());
        for start in &starts {
            if !repo_ids.insert(start.repo_id()) {
                return Err(WatcherSupervisorStartError::DuplicateRepo(start.repo_id()));
            }
        }

        let supervisor = Self::empty(starts.len(), publisher);
        let tracker = Arc::new(BootstrapTracker::new());
        let mut started = Vec::with_capacity(starts.len());
        for start in starts {
            let repo_id = start.repo_id();
            let reservation = match supervisor.reserve_bootstrap(repo_id, start.generation()) {
                Ok(reservation) => reservation,
                Err(_) => {
                    let cleanup = supervisor.shutdown_collect();
                    return Err(WatcherSupervisorStartError::Coordination { cleanup });
                }
            };
            if let Err(error) =
                supervisor.start_reserved_inner(&reservation, start, Some(tracker.clone()))
            {
                let cleanup = supervisor.shutdown_collect();
                return Err(map_start_error(repo_id, error, cleanup));
            }
            if let Err(error) = supervisor.finalize_mounted(&reservation) {
                let cleanup = supervisor.shutdown_collect();
                return Err(map_start_error(repo_id, error, cleanup));
            }
            started.push(repo_id);
        }

        match supervisor.validate_bootstrap(&started) {
            Ok(()) if tracker.seal() => Ok(supervisor),
            Ok(()) => {
                let (repo_id, failure) = tracker.failure.get().cloned().unwrap_or_else(|| {
                    (
                        started.first().copied().unwrap_or_else(RepoId::nil),
                        coordination_failure(
                            "watcher bootstrap seal failed without a recorded repo failure",
                        ),
                    )
                });
                let cleanup = supervisor.shutdown_collect();
                Err(WatcherSupervisorStartError::FailedBeforeMounted {
                    repo_id,
                    failure,
                    cleanup,
                })
            }
            Err((repo_id, failure)) => {
                let cleanup = supervisor.shutdown_collect();
                Err(WatcherSupervisorStartError::FailedBeforeMounted {
                    repo_id,
                    failure,
                    cleanup,
                })
            }
        }
    }

    fn empty(capacity: usize, publisher: WatcherRefreshCallback) -> Self {
        Self {
            slots: Arc::new(Mutex::new(HashMap::with_capacity(capacity))),
            owned: Mutex::new(OwnedHandles {
                handles: HashMap::with_capacity(capacity),
                start_order: Vec::with_capacity(capacity),
                shutting_down: false,
            }),
            publisher,
            #[cfg(test)]
            fail_next_start: AtomicBool::new(false),
            #[cfg(test)]
            fail_next_shutdown_after_cleanup: AtomicBool::new(false),
        }
    }

    #[cfg(test)]
    pub(crate) fn fail_next_start_for_test(&self) {
        self.fail_next_start.store(true, Ordering::Release);
    }

    #[cfg(test)]
    pub(crate) fn fail_next_shutdown_after_cleanup_for_test(&self) {
        self.fail_next_shutdown_after_cleanup
            .store(true, Ordering::Release);
    }

    pub(crate) fn view(&self) -> WatcherRuntimeView {
        WatcherRuntimeView::new(self.slots.clone())
    }

    pub(crate) fn shutdown(&self) -> Result<(), WatcherSupervisorShutdownError> {
        let failures = self.shutdown_collect();
        if failures.is_empty() {
            Ok(())
        } else {
            Err(WatcherSupervisorShutdownError { failures })
        }
    }

    fn validate_bootstrap(&self, start_order: &[RepoId]) -> Result<(), (RepoId, WatcherFailure)> {
        let slots = self.slots.lock().map_err(|_| {
            (
                start_order.first().copied().unwrap_or_else(RepoId::nil),
                coordination_failure("watcher supervisor slot registry poisoned"),
            )
        })?;
        for repo_id in start_order {
            let Some(slot) = slots.get(repo_id) else {
                return Err((
                    *repo_id,
                    coordination_failure("watcher bootstrap slot disappeared before handoff"),
                ));
            };
            if slot.state() != RepoMountState::Mounted {
                return Err((
                    *repo_id,
                    slot.recorded_failure().unwrap_or_else(|| {
                        coordination_failure(format!(
                            "watcher bootstrap final cut found {:?} mount state",
                            slot.state()
                        ))
                    }),
                ));
            }
        }
        Ok(())
    }

    fn shutdown_collect(&self) -> Vec<WatcherFailure> {
        let slots = match self.slots.lock() {
            Ok(slots) => slots.values().cloned().collect::<Vec<_>>(),
            Err(poisoned) => poisoned.into_inner().values().cloned().collect(),
        };
        for slot in &slots {
            slot.begin_shutdown();
            slot.drop_deferred();
        }

        let (mut handles, mut start_order) = match self.owned.lock() {
            Ok(mut owned) => {
                owned.shutting_down = true;
                (
                    std::mem::take(&mut owned.handles),
                    std::mem::take(&mut owned.start_order),
                )
            }
            Err(poisoned) => {
                let mut owned = poisoned.into_inner();
                owned.shutting_down = true;
                (
                    std::mem::take(&mut owned.handles),
                    std::mem::take(&mut owned.start_order),
                )
            }
        };

        let mut failures = Vec::new();
        while let Some(key) = start_order.pop() {
            if let Some(entry) = handles.remove(&key.repo_id)
                && let Err(failure) = entry.handle.shutdown()
            {
                failures.push(failure);
            }
        }
        for (_, entry) in handles {
            if let Err(failure) = entry.handle.shutdown() {
                failures.push(failure);
            }
        }
        for slot in slots {
            slot.mark_stopped_preserving_refresh();
            slot.drop_deferred();
        }
        let final_slots = match self.slots.lock() {
            Ok(slots) => slots.values().cloned().collect::<Vec<_>>(),
            Err(poisoned) => poisoned.into_inner().values().cloned().collect(),
        };
        for slot in final_slots {
            slot.mark_stopped_preserving_refresh();
            slot.drop_deferred();
        }
        failures
    }
}

impl Drop for WatcherSupervisor {
    fn drop(&mut self) {
        for failure in self.shutdown_collect() {
            tracing::error!(
                error = %failure,
                cleanup = ?failure.cleanup,
                "best-effort watcher supervisor shutdown failed during Drop"
            );
        }
    }
}

fn map_start_error(
    repo_id: RepoId,
    error: WatcherLifecycleError,
    cleanup: Vec<WatcherFailure>,
) -> WatcherSupervisorStartError {
    match error {
        WatcherLifecycleError::Start { source, .. } => WatcherSupervisorStartError::Start {
            repo_id,
            source,
            cleanup,
        },
        WatcherLifecycleError::FailedBeforeMounted { failure, .. } => {
            WatcherSupervisorStartError::FailedBeforeMounted {
                repo_id,
                failure: *failure,
                cleanup,
            }
        }
        WatcherLifecycleError::Shutdown { failure, .. } => {
            WatcherSupervisorStartError::FailedBeforeMounted {
                repo_id,
                failure,
                cleanup,
            }
        }
        _ => WatcherSupervisorStartError::Coordination { cleanup },
    }
}

pub(super) fn coordination_failure(primary: impl Into<String>) -> WatcherFailure {
    WatcherFailure::new(
        WatcherFailurePhase::Worker,
        WatcherFailureKind::Coordination,
        primary,
    )
}

#[cfg(test)]
mod bootstrap_tests {
    use super::*;

    #[test]
    fn bootstrap_seal_is_the_single_failure_cut() {
        let before_cut = BootstrapTracker::new();
        before_cut.fail(RepoId::new_v4(), coordination_failure("failed before cut"));
        assert!(!before_cut.seal());

        let after_cut = BootstrapTracker::new();
        assert!(after_cut.seal());
        after_cut.fail(RepoId::new_v4(), coordination_failure("failed after cut"));
        assert_eq!(after_cut.state.load(Ordering::Acquire), BOOTSTRAP_RUNNING);
    }
}
