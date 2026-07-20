//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!   - 03_storage/watcher#watcher-contract
//!
//! Read-only watcher readiness view and exact mount admission token.

use super::slot::{MountSlot, RepoMountState, RuntimeSlots};
use deve_core::models::RepoId;
use deve_core::protocol::RepoReadiness;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use std::sync::atomic::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WatcherRuntimeAggregateStatus {
    Healthy,
    Transitioning,
    Degraded,
    Unknown,
}

impl WatcherRuntimeAggregateStatus {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Transitioning => "transitioning",
            Self::Degraded => "degraded",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct WatcherRuntimeAggregate {
    pub(crate) status: WatcherRuntimeAggregateStatus,
    pub(crate) expected: usize,
    pub(crate) running: usize,
    pub(crate) unavailable: usize,
}

impl WatcherRuntimeAggregate {
    fn unknown(expected: usize) -> Self {
        Self {
            status: WatcherRuntimeAggregateStatus::Unknown,
            expected,
            running: 0,
            unavailable: expected,
        }
    }
}

#[derive(Clone)]
pub(crate) struct WatcherRuntimeView {
    slots: RuntimeSlots,
    #[cfg(test)]
    allow_unknown: bool,
}

impl WatcherRuntimeView {
    pub(super) fn new(slots: RuntimeSlots) -> Self {
        Self {
            slots,
            #[cfg(test)]
            allow_unknown: false,
        }
    }

    pub(crate) fn admit(
        &self,
        repo_id: RepoId,
    ) -> Result<MountAdmissionToken, MountAdmissionError> {
        let slot = self.slot(repo_id)?;
        if slot.state() != RepoMountState::Mounted {
            return Err(MountAdmissionError::Unavailable);
        }
        Ok(MountAdmissionToken {
            repo_id,
            generation: slot.generation,
            slot,
        })
    }

    pub(crate) fn repo_readiness(&self, repo_id: RepoId) -> RepoReadiness {
        let Ok(slot) = self.slot(repo_id) else {
            return RepoReadiness::Unavailable;
        };
        match slot.state() {
            RepoMountState::Mounted => RepoReadiness::Mounted,
            RepoMountState::Starting | RepoMountState::Transitioning => {
                RepoReadiness::Transitioning
            }
            RepoMountState::Failed | RepoMountState::Stopped => RepoReadiness::Unavailable,
        }
    }

    fn slot(&self, repo_id: RepoId) -> Result<Arc<MountSlot>, MountAdmissionError> {
        let slot = {
            let slots = self
                .slots
                .lock()
                .map_err(|_| MountAdmissionError::Unavailable)?;
            slots.get(&repo_id).cloned()
        };
        #[cfg(test)]
        let slot = slot.or_else(|| {
            self.allow_unknown
                .then(|| Arc::new(MountSlot::mounted(repo_id, 1)))
        });
        slot.ok_or(MountAdmissionError::Unavailable)
    }

    pub(crate) fn aggregate(&self, expected_repo_ids: &HashSet<RepoId>) -> WatcherRuntimeAggregate {
        let expected = expected_repo_ids.len();
        let slots = match self.slots.lock() {
            Ok(slots) => slots,
            Err(_) => return WatcherRuntimeAggregate::unknown(expected),
        };
        let Some(states) = stable_expected_states(&slots, expected_repo_ids) else {
            return WatcherRuntimeAggregate::unknown(expected);
        };
        let mut running = 0usize;
        let mut transitioning = false;
        let mut degraded = false;
        for state in states {
            let Some(state) = state else {
                degraded = true;
                continue;
            };
            match state {
                RepoMountState::Mounted => running += 1,
                RepoMountState::Starting | RepoMountState::Transitioning => transitioning = true,
                RepoMountState::Failed | RepoMountState::Stopped => degraded = true,
            }
        }
        let status = if degraded {
            WatcherRuntimeAggregateStatus::Degraded
        } else if transitioning {
            WatcherRuntimeAggregateStatus::Transitioning
        } else {
            WatcherRuntimeAggregateStatus::Healthy
        };
        WatcherRuntimeAggregate {
            status,
            expected,
            running,
            unavailable: expected.saturating_sub(running),
        }
    }

    #[cfg(not(test))]
    pub(crate) fn same_runtime(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.slots, &other.slots)
    }

    #[cfg(test)]
    pub(crate) fn permissive_for_tests() -> Self {
        Self {
            slots: Arc::new(Mutex::new(HashMap::new())),
            allow_unknown: true,
        }
    }

    #[cfg(test)]
    pub(crate) fn with_state_for_test(
        repo_id: RepoId,
        generation: u64,
        state: RepoMountState,
    ) -> Self {
        let slot = Arc::new(MountSlot::mounted(repo_id, generation));
        slot.state.store(state.encode(), Ordering::Release);
        let slots = HashMap::from([(repo_id, slot)]);
        Self {
            slots: Arc::new(Mutex::new(slots)),
            allow_unknown: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn set_state_for_test(&self, repo_id: RepoId, state: RepoMountState) {
        let slots = self.slots.lock().expect("watcher runtime test slots");
        let slot = slots.get(&repo_id).expect("watcher runtime test slot");
        slot.state.store(state.encode(), Ordering::Release);
    }

    #[cfg(test)]
    pub(crate) fn insert_state_for_test(
        &self,
        repo_id: RepoId,
        generation: u64,
        state: RepoMountState,
    ) {
        let slot = Arc::new(MountSlot::mounted(repo_id, generation));
        slot.state.store(state.encode(), Ordering::Release);
        self.slots
            .lock()
            .expect("watcher runtime test slots")
            .insert(repo_id, slot);
    }

    #[cfg(test)]
    pub(crate) fn poison_slots_for_test(&self) {
        let slots = self.slots.clone();
        let _ = std::thread::spawn(move || {
            let _guard = slots.lock().expect("watcher runtime test slots");
            panic!("poison watcher runtime test slots");
        })
        .join();
    }
}

fn stable_expected_states(
    slots: &HashMap<RepoId, Arc<MountSlot>>,
    expected_repo_ids: &HashSet<RepoId>,
) -> Option<Vec<Option<RepoMountState>>> {
    const MAX_SNAPSHOT_ATTEMPTS: usize = 3;
    for _ in 0..MAX_SNAPSHOT_ATTEMPTS {
        let first = expected_states(slots, expected_repo_ids);
        let second = expected_states(slots, expected_repo_ids);
        if first == second {
            return Some(first);
        }
    }
    None
}

fn expected_states(
    slots: &HashMap<RepoId, Arc<MountSlot>>,
    expected_repo_ids: &HashSet<RepoId>,
) -> Vec<Option<RepoMountState>> {
    let mut repo_ids = expected_repo_ids.iter().copied().collect::<Vec<_>>();
    repo_ids.sort_unstable_by_key(|repo_id| repo_id.as_u128());
    repo_ids
        .into_iter()
        .map(|repo_id| {
            slots
                .get(&repo_id)
                .filter(|slot| slot.repo_id == repo_id)
                .map(|slot| slot.state())
        })
        .collect()
}

pub(crate) struct MountAdmissionToken {
    repo_id: RepoId,
    generation: u64,
    slot: Arc<MountSlot>,
}

impl MountAdmissionToken {
    pub(crate) fn repo_id(&self) -> RepoId {
        self.repo_id
    }

    pub(crate) fn revalidate(&self) -> Result<(), MountAdmissionError> {
        if self.slot.repo_id == self.repo_id
            && self.slot.generation == self.generation
            && self.slot.state() == RepoMountState::Mounted
        {
            Ok(())
        } else {
            Err(MountAdmissionError::Unavailable)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MountAdmissionError {
    Unavailable,
}

impl fmt::Display for MountAdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("workspace ingestion is unavailable")
    }
}

impl std::error::Error for MountAdmissionError {}
