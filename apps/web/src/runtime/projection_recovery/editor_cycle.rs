use deve_core::protocol::ProjectionRecoveryRequired;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryStart {
    ReopenNow,
    TrailingQueued,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryCompletion {
    NotActive,
    Finished,
    ReopenTrailing(ProjectionRecoveryRequired),
}

#[derive(Default)]
struct RecoveryCycle {
    active: Option<ProjectionRecoveryRequired>,
    active_generation: Option<u64>,
    trailing: Option<ProjectionRecoveryRequired>,
}

/// Per-editor recovery cycle. Every invalidation received while a generation
/// is active records one bounded trailing reopen. Typed recovery messages do
/// not carry a source waterline, so value equality is not proof of duplication.
#[derive(Clone, Default)]
pub struct ProjectionRecoveryCoordinator {
    cycle: Arc<Mutex<RecoveryCycle>>,
}

impl ProjectionRecoveryCoordinator {
    pub fn begin(&self, required: ProjectionRecoveryRequired) -> RecoveryStart {
        let mut cycle = self
            .cycle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if cycle.active.is_none() {
            cycle.active = Some(required);
            cycle.active_generation = None;
            return RecoveryStart::ReopenNow;
        }
        cycle.trailing = Some(required);
        RecoveryStart::TrailingQueued
    }

    pub fn mark_generation(&self, generation: u64) {
        let mut cycle = self
            .cycle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if cycle.active.is_some() {
            cycle.active_generation = Some(generation);
        }
    }

    pub fn finish_generation(&self, generation: u64) -> RecoveryCompletion {
        let mut cycle = self
            .cycle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if cycle.active.is_none() || cycle.active_generation != Some(generation) {
            return RecoveryCompletion::NotActive;
        }
        if let Some(trailing) = cycle.trailing.take() {
            cycle.active = Some(trailing.clone());
            cycle.active_generation = None;
            return RecoveryCompletion::ReopenTrailing(trailing);
        }
        cycle.active = None;
        cycle.active_generation = None;
        RecoveryCompletion::Finished
    }

    pub fn is_active(&self) -> bool {
        self.cycle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .active
            .is_some()
    }

    pub fn reset(&self) {
        *self
            .cycle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = RecoveryCycle::default();
    }
}
