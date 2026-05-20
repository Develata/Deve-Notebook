//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-process-adapter-decision

use deve_core::native_adapter::{
    NativeEndpointReady, NativeProcessAdapterPolicy, NativeProcessExitStatus,
    NativeProcessRuntimeError, NativeProcessRuntimeEvent, NativeProcessRuntimeFailureKind,
    NativeProcessRuntimeHandle, NativeProcessRuntimeSnapshot, NativeProcessRuntimeState,
    NativeProcessSpawnSpec, NativeServiceHealthProbe,
};

#[derive(Debug, Clone)]
pub(super) struct DesktopProcessRuntimeCore {
    policy: NativeProcessAdapterPolicy,
    snapshot: NativeProcessRuntimeSnapshot,
    events: Vec<NativeProcessRuntimeEvent>,
    restart_attempt: u32,
    max_restart_attempts: u32,
}

impl DesktopProcessRuntimeCore {
    pub(super) fn new(policy: NativeProcessAdapterPolicy, max_restart_attempts: u32) -> Self {
        Self {
            policy,
            snapshot: NativeProcessRuntimeSnapshot::disabled_by_policy(policy),
            events: Vec::new(),
            restart_attempt: 0,
            max_restart_attempts,
        }
    }

    pub(super) fn is_running(&self) -> bool {
        self.snapshot.handle.is_some()
    }

    pub(super) fn request_start(
        &mut self,
        spec: &NativeProcessSpawnSpec,
        timestamp_unix_ms: i64,
    ) -> Result<NativeProcessRuntimeSnapshot, NativeProcessRuntimeError> {
        if !self.policy.child_process_runtime_enabled {
            return Err(NativeProcessRuntimeError::RuntimeDisabled);
        }
        spec.validate_contract()?;
        self.transition(NativeProcessRuntimeState::SpawnRequested, timestamp_unix_ms);
        Ok(self.snapshot())
    }

    pub(super) fn record_started(
        &mut self,
        handle: NativeProcessRuntimeHandle,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.snapshot.handle = Some(handle);
        self.snapshot.started_at_unix_ms = Some(timestamp_unix_ms);
        self.transition(NativeProcessRuntimeState::Spawned, timestamp_unix_ms);
        self.snapshot()
    }

    pub(super) fn record_endpoint_probe(
        &mut self,
        endpoint: NativeEndpointReady,
        probe: NativeServiceHealthProbe,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.snapshot.endpoint = Some(endpoint);
        self.snapshot.health_probe = probe;
        if probe.is_healthy() {
            self.transition(
                NativeProcessRuntimeState::EndpointHealthy,
                timestamp_unix_ms,
            );
        } else {
            self.record_failure(
                NativeProcessRuntimeFailureKind::HealthProbeFailed,
                timestamp_unix_ms,
            );
        }
        self.snapshot()
    }

    pub(super) fn record_session_handoff(
        &mut self,
        session_bound: bool,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        if !session_bound {
            self.record_failure(
                NativeProcessRuntimeFailureKind::SessionHandoffFailed,
                timestamp_unix_ms,
            );
            return self.snapshot();
        }
        if let Some(endpoint) = self.snapshot.endpoint.as_mut() {
            endpoint.session_bound = true;
        }
        self.transition(
            NativeProcessRuntimeState::SessionHandoffReady,
            timestamp_unix_ms,
        );
        self.snapshot()
    }

    pub(super) fn mark_runtime_ready(
        &mut self,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.transition(NativeProcessRuntimeState::RuntimeReady, timestamp_unix_ms);
        self.snapshot()
    }

    pub(super) fn record_process_exit(
        &mut self,
        status: NativeProcessExitStatus,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.snapshot.handle = None;
        self.snapshot.exit_status = Some(status);
        self.record_failure(
            NativeProcessRuntimeFailureKind::ProcessExited,
            timestamp_unix_ms,
        );
        self.snapshot()
    }

    pub(super) fn record_stopped(
        &mut self,
        status: Option<NativeProcessExitStatus>,
        timestamp_unix_ms: i64,
    ) -> NativeProcessRuntimeSnapshot {
        self.snapshot.handle = None;
        self.snapshot.exit_status = status;
        self.transition(NativeProcessRuntimeState::Stopped, timestamp_unix_ms);
        self.snapshot()
    }

    pub(super) fn snapshot(&self) -> NativeProcessRuntimeSnapshot {
        self.snapshot.clone()
    }

    pub(super) fn events(&self) -> &[NativeProcessRuntimeEvent] {
        &self.events
    }

    pub(super) fn record_failure(
        &mut self,
        failure: NativeProcessRuntimeFailureKind,
        timestamp_unix_ms: i64,
    ) {
        let retryable =
            failure.retryable_by_default() && self.restart_attempt < self.max_restart_attempts;
        if retryable {
            self.restart_attempt += 1;
            self.transition(NativeProcessRuntimeState::Restarting, timestamp_unix_ms);
        } else {
            self.transition(NativeProcessRuntimeState::Offline, timestamp_unix_ms);
        }
        self.snapshot.last_failure = Some(failure);
        if let Some(event) = self.events.last_mut() {
            event.failure = Some(failure);
        }
    }

    fn transition(&mut self, state: NativeProcessRuntimeState, timestamp_unix_ms: i64) {
        self.snapshot.state = state;
        self.events.push(NativeProcessRuntimeEvent {
            state,
            timestamp_unix_ms,
            failure: None,
        });
    }
}
