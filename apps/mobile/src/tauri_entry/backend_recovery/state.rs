//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-shell-modes
//!   - 11_ui_design/03_mobile#mobile-service-supervisor-contract
//!
//! Process-local recovery transaction state. This state owns no repository or
//! writer authority; it only guards the native shell lifecycle and exposes a
//! typed diagnostic snapshot to the trusted bundled LocalBackend WebView.

use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MobileBackendRecoveryPhase {
    Idle,
    RemoteActive,
    CandidateStarted,
    NativeControlRetired,
    RemoteSurfaceRetired,
    PreferenceCommitted,
    LocalPluginsRegistered,
    SupervisorManaged,
    LocalWindowCreated,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MobileBackendRecoverySnapshot {
    pub recovery_id: u64,
    pub phase: MobileBackendRecoveryPhase,
    pub remote_surface_retired: bool,
    pub preference_committed_after_remote_retirement: bool,
    pub local_plugins_registered_after_remote_retirement: bool,
    pub supervisor_managed: bool,
    pub local_window_created: bool,
    pub active_runtime_owners: u8,
    pub last_error: Option<String>,
}

impl Default for MobileBackendRecoverySnapshot {
    fn default() -> Self {
        Self {
            recovery_id: 0,
            phase: MobileBackendRecoveryPhase::Idle,
            remote_surface_retired: false,
            preference_committed_after_remote_retirement: false,
            local_plugins_registered_after_remote_retirement: false,
            supervisor_managed: false,
            local_window_created: false,
            active_runtime_owners: 0,
            last_error: None,
        }
    }
}

pub(crate) struct MobileBackendRecoveryState {
    active: AtomicBool,
    next_recovery_id: AtomicU64,
    snapshot: Mutex<MobileBackendRecoverySnapshot>,
}

impl Default for MobileBackendRecoveryState {
    fn default() -> Self {
        Self {
            active: AtomicBool::new(false),
            next_recovery_id: AtomicU64::new(0),
            snapshot: Mutex::new(MobileBackendRecoverySnapshot::default()),
        }
    }
}

impl MobileBackendRecoveryState {
    pub(crate) fn begin(&self) -> Result<Option<u64>, String> {
        if self
            .active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Ok(None);
        }
        let recovery_id = match self.next_recovery_id.fetch_update(
            Ordering::AcqRel,
            Ordering::Acquire,
            |current| current.checked_add(1),
        ) {
            Ok(previous) => previous + 1,
            Err(_) => {
                self.active.store(false, Ordering::Release);
                return Err("mobile backend recovery id overflow".to_string());
            }
        };
        let mut snapshot = match self.lock_snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.active.store(false, Ordering::Release);
                return Err(error);
            }
        };
        *snapshot = MobileBackendRecoverySnapshot {
            phase: MobileBackendRecoveryPhase::RemoteActive,
            ..MobileBackendRecoverySnapshot::default()
        };
        snapshot.recovery_id = recovery_id;
        Ok(Some(recovery_id))
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub(crate) fn record_phase(
        &self,
        recovery_id: u64,
        phase: MobileBackendRecoveryPhase,
    ) -> Result<(), String> {
        let mut snapshot = self.lock_snapshot()?;
        if !self.active.load(Ordering::Acquire) || snapshot.recovery_id != recovery_id {
            return Err("stale mobile backend recovery transition".to_string());
        }
        if !valid_transition(snapshot.phase, phase) {
            return Err(format!(
                "invalid mobile backend recovery transition {:?} -> {:?}",
                snapshot.phase, phase
            ));
        }
        snapshot.phase = phase;
        match phase {
            MobileBackendRecoveryPhase::CandidateStarted => {
                snapshot.active_runtime_owners = 1;
            }
            MobileBackendRecoveryPhase::RemoteSurfaceRetired => {
                snapshot.remote_surface_retired = true;
            }
            MobileBackendRecoveryPhase::PreferenceCommitted => {
                snapshot.preference_committed_after_remote_retirement =
                    snapshot.remote_surface_retired;
            }
            MobileBackendRecoveryPhase::LocalPluginsRegistered => {
                snapshot.local_plugins_registered_after_remote_retirement =
                    snapshot.remote_surface_retired;
            }
            MobileBackendRecoveryPhase::SupervisorManaged => {
                snapshot.supervisor_managed = true;
            }
            MobileBackendRecoveryPhase::LocalWindowCreated => {
                snapshot.local_window_created = true;
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn finish_success(&self, recovery_id: u64) -> Result<(), String> {
        let snapshot = self.lock_snapshot()?;
        if snapshot.recovery_id != recovery_id
            || snapshot.phase != MobileBackendRecoveryPhase::LocalWindowCreated
            || snapshot.active_runtime_owners != 1
        {
            return Err("mobile backend recovery completed from an invalid state".to_string());
        }
        drop(snapshot);
        self.active.store(false, Ordering::Release);
        Ok(())
    }

    pub(crate) fn finish_failure(
        &self,
        recovery_id: u64,
        error: impl Into<String>,
        active_runtime_owners: u8,
    ) -> Result<(), String> {
        let mut snapshot = self.lock_snapshot()?;
        if snapshot.recovery_id != recovery_id {
            return Err("stale mobile backend recovery failure".to_string());
        }
        snapshot.phase = MobileBackendRecoveryPhase::Failed;
        snapshot.active_runtime_owners = active_runtime_owners;
        snapshot.last_error = Some(error.into());
        drop(snapshot);
        self.active.store(false, Ordering::Release);
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> Result<MobileBackendRecoverySnapshot, String> {
        Ok(self.lock_snapshot()?.clone())
    }

    pub(crate) fn force_inactive(&self) {
        self.active.store(false, Ordering::Release);
    }

    fn lock_snapshot(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, MobileBackendRecoverySnapshot>, String> {
        self.snapshot
            .lock()
            .map_err(|_| "mobile backend recovery state poisoned".to_string())
    }
}

fn valid_transition(current: MobileBackendRecoveryPhase, next: MobileBackendRecoveryPhase) -> bool {
    matches!(
        (current, next),
        (
            MobileBackendRecoveryPhase::RemoteActive,
            MobileBackendRecoveryPhase::CandidateStarted
        ) | (
            MobileBackendRecoveryPhase::CandidateStarted,
            MobileBackendRecoveryPhase::NativeControlRetired
        ) | (
            MobileBackendRecoveryPhase::NativeControlRetired,
            MobileBackendRecoveryPhase::RemoteSurfaceRetired
        ) | (
            MobileBackendRecoveryPhase::RemoteSurfaceRetired,
            MobileBackendRecoveryPhase::PreferenceCommitted
        ) | (
            MobileBackendRecoveryPhase::PreferenceCommitted,
            MobileBackendRecoveryPhase::LocalPluginsRegistered
        ) | (
            MobileBackendRecoveryPhase::LocalPluginsRegistered,
            MobileBackendRecoveryPhase::SupervisorManaged
        ) | (
            MobileBackendRecoveryPhase::SupervisorManaged,
            MobileBackendRecoveryPhase::LocalWindowCreated
        )
    )
}

#[cfg(test)]
mod tests {
    use super::{MobileBackendRecoveryPhase, MobileBackendRecoveryState};

    #[test]
    fn recovery_state_is_single_flight_and_records_ordered_success() {
        let state = MobileBackendRecoveryState::default();
        let id = state.begin().expect("begin").expect("first attempt");
        assert_eq!(state.begin().expect("second begin"), None);
        for phase in [
            MobileBackendRecoveryPhase::CandidateStarted,
            MobileBackendRecoveryPhase::NativeControlRetired,
            MobileBackendRecoveryPhase::RemoteSurfaceRetired,
            MobileBackendRecoveryPhase::PreferenceCommitted,
            MobileBackendRecoveryPhase::LocalPluginsRegistered,
            MobileBackendRecoveryPhase::SupervisorManaged,
            MobileBackendRecoveryPhase::LocalWindowCreated,
        ] {
            state.record_phase(id, phase).expect("ordered transition");
        }
        state.finish_success(id).expect("finish success");
        let snapshot = state.snapshot().expect("snapshot");
        assert!(snapshot.remote_surface_retired);
        assert!(snapshot.preference_committed_after_remote_retirement);
        assert!(snapshot.local_plugins_registered_after_remote_retirement);
        assert_eq!(snapshot.active_runtime_owners, 1);
        assert!(!state.is_active());
    }

    #[test]
    fn recovery_state_rejects_skipped_phase_and_clears_runtime_on_failure() {
        let state = MobileBackendRecoveryState::default();
        let id = state.begin().expect("begin").expect("attempt");
        assert!(
            state
                .record_phase(id, MobileBackendRecoveryPhase::RemoteSurfaceRetired)
                .is_err()
        );
        state
            .finish_failure(id, "candidate stopped", 0)
            .expect("finish failure");
        let snapshot = state.snapshot().expect("snapshot");
        assert_eq!(snapshot.phase, MobileBackendRecoveryPhase::Failed);
        assert_eq!(snapshot.active_runtime_owners, 0);
        assert_eq!(snapshot.last_error.as_deref(), Some("candidate stopped"));
        assert!(state.begin().expect("retry begin").is_some());
    }
}
