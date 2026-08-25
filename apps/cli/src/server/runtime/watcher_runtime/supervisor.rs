//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 04_repository#repo-health-and-repair
//!   - 07_network#server-ws-runtime
//!
//! Unique server owner of watcher handles, slots and generations.

use super::error::WatcherSupervisorShutdownError;
use super::slot::RuntimeSlots;
use super::view::WatcherRuntimeView;
use deve_core::models::RepoId;
use deve_core::sync::watcher::{
    RepoWatcherHandle, WatcherFailure, WatcherFailureKind, WatcherFailurePhase,
    WatcherRefreshCallback,
};
use std::collections::HashMap;
#[cfg(test)]
use std::sync::atomic::AtomicBool;
#[cfg(test)]
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

impl WatcherSupervisor {
    pub(super) fn empty(capacity: usize, publisher: WatcherRefreshCallback) -> Self {
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
        shutdown_result(failures)
    }

    pub(crate) fn shutdown_bounded(
        &self,
        timeout: Duration,
    ) -> Result<(), WatcherSupervisorShutdownError> {
        let started = Instant::now();
        let deadline = started.checked_add(timeout).unwrap_or(started);
        let failures = self.shutdown_collect_with(|handle| {
            handle.shutdown_bounded(deadline.saturating_duration_since(Instant::now()))
        });
        shutdown_result(failures)
    }

    pub(super) fn shutdown_collect(&self) -> Vec<WatcherFailure> {
        self.shutdown_collect_with(RepoWatcherHandle::shutdown)
    }

    fn shutdown_collect_with(
        &self,
        mut shutdown_handle: impl FnMut(RepoWatcherHandle) -> Result<(), WatcherFailure>,
    ) -> Vec<WatcherFailure> {
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
                && let Err(failure) = shutdown_handle(entry.handle)
            {
                failures.push(failure);
            }
        }
        for (_, entry) in handles {
            if let Err(failure) = shutdown_handle(entry.handle) {
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

fn shutdown_result(failures: Vec<WatcherFailure>) -> Result<(), WatcherSupervisorShutdownError> {
    if failures.is_empty() {
        Ok(())
    } else {
        Err(WatcherSupervisorShutdownError { failures })
    }
}

impl Drop for WatcherSupervisor {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown_bounded(Duration::from_millis(250)) {
            for failure in error.failures {
                tracing::error!(
                    error = %failure,
                    cleanup = ?failure.cleanup,
                    "bounded watcher supervisor shutdown failed during Drop"
                );
            }
        }
    }
}

pub(super) fn coordination_failure(primary: impl Into<String>) -> WatcherFailure {
    WatcherFailure::new(
        WatcherFailurePhase::Worker,
        WatcherFailureKind::Coordination,
        primary,
    )
}
