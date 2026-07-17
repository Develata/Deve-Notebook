//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 07_network#server-ws-runtime
//!
//! Unique server owner of watcher handles, slots and generations.

use super::slot::{MountSlot, RepoMountState, RuntimeSlots, WatcherRuntimeView};
use deve_core::models::RepoId;
use deve_core::sync::watcher::{
    RepoWatcherHandle, RepoWatcherStart, WatcherFailure, WatcherStartError,
};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

const BOOTSTRAP_BUILDING: u8 = 0;
const BOOTSTRAP_RUNNING: u8 = 1;
const BOOTSTRAP_FAILED: u8 = 2;

pub(crate) struct WatcherSupervisor {
    slots: RuntimeSlots,
    handles: HashMap<RepoId, RepoWatcherHandle>,
    start_order: Vec<RepoId>,
}

impl WatcherSupervisor {
    pub(crate) fn start_all(
        starts: Vec<RepoWatcherStart>,
    ) -> Result<Self, WatcherSupervisorStartError> {
        let mut repo_ids = HashSet::with_capacity(starts.len());
        for start in &starts {
            if !repo_ids.insert(start.repo_id()) {
                return Err(WatcherSupervisorStartError::DuplicateRepo(start.repo_id()));
            }
        }

        let slots = Arc::new(Mutex::new(HashMap::with_capacity(starts.len())));
        let bootstrap_state = Arc::new(AtomicU8::new(BOOTSTRAP_BUILDING));
        let bootstrap_failure = Arc::new(OnceLock::new());
        let mut supervisor = Self {
            slots: slots.clone(),
            handles: HashMap::with_capacity(starts.len()),
            start_order: Vec::with_capacity(starts.len()),
        };

        for start in starts {
            let repo_id = start.repo_id();
            let generation = start.generation();
            let slot = Arc::new(MountSlot::starting(repo_id, generation));
            let inserted = match slots.lock() {
                Ok(mut slots) => {
                    slots.insert(repo_id, slot.clone());
                    true
                }
                Err(_) => false,
            };
            if !inserted {
                let cleanup = supervisor.shutdown_started();
                return Err(WatcherSupervisorStartError::Coordination { cleanup });
            }
            let failure_slot = slot.clone();
            let failure_bootstrap_state = bootstrap_state.clone();
            let failure_bootstrap = bootstrap_failure.clone();
            let start = start.with_failure_callback(Arc::new(move |failure| {
                let _ = failure_bootstrap.set((repo_id, failure.clone()));
                let _ = failure_bootstrap_state.compare_exchange(
                    BOOTSTRAP_BUILDING,
                    BOOTSTRAP_FAILED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                );
                failure_slot.fail(failure);
            }));
            let handle = match RepoWatcherHandle::start(start) {
                Ok(handle) => handle,
                Err(source) => {
                    let cleanup = supervisor.shutdown_started();
                    return Err(WatcherSupervisorStartError::Start {
                        repo_id,
                        source,
                        cleanup,
                    });
                }
            };
            supervisor.start_order.push(repo_id);
            supervisor.handles.insert(repo_id, handle);
            if let Err(failure) = slot.mark_mounted() {
                let cleanup = supervisor.shutdown_started();
                return Err(WatcherSupervisorStartError::FailedBeforeMounted {
                    repo_id,
                    failure,
                    cleanup,
                });
            }
        }
        let bootstrap_validation = match supervisor.slots.lock() {
            Ok(slots) => Some(validate_bootstrap_slots(&supervisor.start_order, &slots)),
            Err(_) => None,
        };
        let (repo_id, failure) = match bootstrap_validation {
            Some(Ok(())) if seal_bootstrap(&bootstrap_state) => return Ok(supervisor),
            Some(Err(failure)) => failure,
            Some(Ok(())) => bootstrap_failure.get().cloned().unwrap_or_else(|| {
                (
                    supervisor
                        .start_order
                        .first()
                        .copied()
                        .unwrap_or_else(RepoId::nil),
                    WatcherFailure::new(
                        deve_core::sync::watcher::WatcherFailurePhase::Worker,
                        deve_core::sync::watcher::WatcherFailureKind::Coordination,
                        "watcher bootstrap seal failed without a recorded repo failure",
                    ),
                )
            }),
            None => {
                let cleanup = supervisor.shutdown_started();
                return Err(WatcherSupervisorStartError::Coordination { cleanup });
            }
        };
        let cleanup = supervisor.shutdown_started();
        Err(WatcherSupervisorStartError::FailedBeforeMounted {
            repo_id,
            failure,
            cleanup,
        })
    }

    pub(crate) fn view(&self) -> WatcherRuntimeView {
        WatcherRuntimeView::new(self.slots.clone())
    }

    pub(crate) fn shutdown(mut self) -> Result<(), WatcherSupervisorShutdownError> {
        let failures = self.shutdown_started();
        if failures.is_empty() {
            Ok(())
        } else {
            Err(WatcherSupervisorShutdownError { failures })
        }
    }

    fn shutdown_started(&mut self) -> Vec<WatcherFailure> {
        let slots = match self.slots.lock() {
            Ok(slots) => slots,
            Err(poisoned) => poisoned.into_inner(),
        };
        for repo_id in &self.start_order {
            if let Some(slot) = slots.get(repo_id) {
                slot.begin_shutdown();
            }
        }
        drop(slots);

        let mut failures = Vec::new();
        while let Some(repo_id) = self.start_order.pop() {
            if let Some(handle) = self.handles.remove(&repo_id)
                && let Err(failure) = handle.shutdown()
            {
                failures.push(failure);
            }
            let slots = match self.slots.lock() {
                Ok(slots) => slots,
                Err(poisoned) => poisoned.into_inner(),
            };
            if let Some(slot) = slots.get(&repo_id) {
                slot.mark_stopped();
            }
        }
        failures
    }
}

impl Drop for WatcherSupervisor {
    fn drop(&mut self) {
        for failure in self.shutdown_started() {
            tracing::error!(
                error = %failure,
                cleanup = ?failure.cleanup,
                "best-effort watcher supervisor shutdown failed during Drop"
            );
        }
    }
}

#[derive(Debug)]
pub(crate) enum WatcherSupervisorStartError {
    DuplicateRepo(RepoId),
    Coordination {
        cleanup: Vec<WatcherFailure>,
    },
    Start {
        repo_id: RepoId,
        source: WatcherStartError,
        cleanup: Vec<WatcherFailure>,
    },
    FailedBeforeMounted {
        repo_id: RepoId,
        failure: WatcherFailure,
        cleanup: Vec<WatcherFailure>,
    },
}

impl fmt::Display for WatcherSupervisorStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateRepo(repo_id) => {
                write!(
                    formatter,
                    "duplicate watcher reservation for repo {repo_id}"
                )
            }
            Self::Coordination { cleanup } => {
                formatter.write_str("watcher supervisor slot registry poisoned")?;
                write_failures(formatter, cleanup)
            }
            Self::Start {
                repo_id,
                source,
                cleanup,
            } => {
                write!(
                    formatter,
                    "watcher start failed for repo {repo_id}: {source}"
                )?;
                write_failures(formatter, cleanup)
            }
            Self::FailedBeforeMounted {
                repo_id,
                failure,
                cleanup,
            } => {
                write!(
                    formatter,
                    "watcher failed before mount handoff for repo {repo_id}: {failure}"
                )?;
                write_failures(formatter, cleanup)
            }
        }
    }
}

impl std::error::Error for WatcherSupervisorStartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Start { source, .. } => Some(source),
            Self::FailedBeforeMounted { failure, .. } => Some(failure),
            Self::DuplicateRepo(_) | Self::Coordination { .. } => None,
        }
    }
}

#[derive(Debug)]
pub(crate) struct WatcherSupervisorShutdownError {
    failures: Vec<WatcherFailure>,
}

impl fmt::Display for WatcherSupervisorShutdownError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("watcher supervisor shutdown failed")?;
        write_failures(formatter, &self.failures)
    }
}

impl std::error::Error for WatcherSupervisorShutdownError {}

fn write_failures(formatter: &mut fmt::Formatter<'_>, failures: &[WatcherFailure]) -> fmt::Result {
    for failure in failures {
        write!(formatter, "; cleanup failure: {failure}")?;
    }
    Ok(())
}

fn validate_bootstrap_slots(
    start_order: &[RepoId],
    slots: &HashMap<RepoId, Arc<MountSlot>>,
) -> Result<(), (RepoId, WatcherFailure)> {
    for repo_id in start_order {
        let Some(slot) = slots.get(repo_id) else {
            return Err((
                *repo_id,
                WatcherFailure::new(
                    deve_core::sync::watcher::WatcherFailurePhase::Worker,
                    deve_core::sync::watcher::WatcherFailureKind::Coordination,
                    "watcher bootstrap slot disappeared before handoff",
                ),
            ));
        };
        if slot.state() == RepoMountState::Mounted {
            continue;
        }
        let failure = slot.recorded_failure().unwrap_or_else(|| {
            WatcherFailure::new(
                deve_core::sync::watcher::WatcherFailurePhase::Worker,
                deve_core::sync::watcher::WatcherFailureKind::Coordination,
                format!(
                    "watcher bootstrap final cut found {:?} mount state",
                    slot.state()
                ),
            )
        });
        return Err((*repo_id, failure));
    }
    Ok(())
}

fn seal_bootstrap(state: &AtomicU8) -> bool {
    state
        .compare_exchange(
            BOOTSTRAP_BUILDING,
            BOOTSTRAP_RUNNING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_ok()
}

#[cfg(test)]
mod bootstrap_tests {
    use super::*;
    use deve_core::sync::watcher::{WatcherFailureKind, WatcherFailurePhase};

    #[test]
    fn bootstrap_final_cut_rejects_a_repo_that_failed_after_mount_handoff() {
        let first = RepoId::new_v4();
        let second = RepoId::new_v4();
        let first_slot = Arc::new(MountSlot::starting(first, 1));
        first_slot.mark_mounted().expect("first mount handoff");
        first_slot.fail(WatcherFailure::new(
            WatcherFailurePhase::Receive,
            WatcherFailureKind::Backend,
            "failed while the next repo started",
        ));
        let second_slot = Arc::new(MountSlot::starting(second, 1));
        second_slot.mark_mounted().expect("second mount handoff");
        let slots = HashMap::from([(first, first_slot), (second, second_slot)]);

        let (failed_repo, failure) = validate_bootstrap_slots(&[first, second], &slots)
            .expect_err("final cut must fail the batch");

        assert_eq!(failed_repo, first);
        assert_eq!(failure.phase, WatcherFailurePhase::Receive);
        assert!(failure.primary.contains("next repo"));
    }

    #[test]
    fn bootstrap_seal_is_the_single_failure_cut() {
        let before_cut = AtomicU8::new(BOOTSTRAP_FAILED);
        assert!(!seal_bootstrap(&before_cut));

        let after_cut = AtomicU8::new(BOOTSTRAP_BUILDING);
        assert!(seal_bootstrap(&after_cut));
        assert_eq!(after_cut.load(Ordering::Acquire), BOOTSTRAP_RUNNING);
        assert_eq!(
            after_cut.compare_exchange(
                BOOTSTRAP_BUILDING,
                BOOTSTRAP_FAILED,
                Ordering::AcqRel,
                Ordering::Acquire,
            ),
            Err(BOOTSTRAP_RUNNING),
            "a failure after the seal belongs to the running runtime"
        );
    }
}
