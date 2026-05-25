//! plan_ref:
//!   - 11_ui_design_03_mobile#mobile-service-supervisor-contract

use deve_core::native_adapter::{
    NativeProcessAdapterError, NativeProcessAdapterSnapshot, NativeServiceFailureKind,
    NativeServiceOffline, NativeServiceSupervisorError, NativeServiceSupervisorObservation,
};

use super::MobileShell;
use crate::types::{MobileServiceState, MobileShellError};

impl MobileShell {
    pub fn mark_probe_timeout(&mut self) -> Result<(), MobileShellError> {
        let process_snapshot = self.core.record_probe_timeout();
        let result = self.observe_process_snapshot(&process_snapshot);
        self.core.clear_shell_runtime_binding();
        result
    }

    pub fn mark_process_shutdown(&mut self) -> Result<(), MobileShellError> {
        let process_snapshot = self.core.record_process_stopped();
        let result = self.observe_process_snapshot(&process_snapshot);
        self.core.clear_shell_runtime_binding();
        result
    }

    pub fn mark_service_offline(&mut self, reason: impl Into<String>, retryable: bool) {
        let offline = self.core.record_service_offline(reason, retryable);
        self.set_offline_state(&offline);
        self.core.clear_runtime_binding();
    }

    pub fn mark_supervisor_failure(
        &mut self,
        kind: NativeServiceFailureKind,
        reason: impl Into<String>,
    ) {
        let offline = self.core.record_failure(kind, reason);
        self.set_offline_state(&offline);
        self.core.clear_runtime_binding();
    }

    pub(super) fn ensure_not_terminal_offline(&self) -> Result<(), MobileShellError> {
        let Some(reason) = self.core.terminal_offline_reason() else {
            return Ok(());
        };
        Err(MobileShellError::ServiceOffline { reason })
    }

    fn set_offline_state(&mut self, offline: &NativeServiceOffline) {
        self.state = if offline.retryable {
            MobileServiceState::ServiceRestarting
        } else {
            MobileServiceState::ServiceOffline
        };
        self.suspended = None;
    }

    pub(super) fn observe_process_snapshot(
        &mut self,
        process_snapshot: &NativeProcessAdapterSnapshot,
    ) -> Result<(), MobileShellError> {
        let observation = self.core.observe_process_snapshot(process_snapshot);
        self.apply_supervisor_observation(observation)
    }

    fn apply_supervisor_observation(
        &mut self,
        observation: NativeServiceSupervisorObservation,
    ) -> Result<(), MobileShellError> {
        match observation {
            NativeServiceSupervisorObservation::EndpointHealthy
            | NativeServiceSupervisorObservation::SessionHandoffReady => Ok(()),
            NativeServiceSupervisorObservation::Offline(offline) => {
                let reason = offline.reason.clone();
                self.set_offline_state(&offline);
                Err(MobileShellError::ServiceOffline { reason })
            }
            NativeServiceSupervisorObservation::Idle => Err(MobileShellError::Supervisor(
                NativeServiceSupervisorError::EndpointNotHealthy,
            )),
        }
    }

    pub(super) fn map_process_adapter_error(error: NativeProcessAdapterError) -> MobileShellError {
        match error {
            NativeProcessAdapterError::InvalidEndpoint(error) => {
                MobileShellError::InvalidEndpoint(error)
            }
            NativeProcessAdapterError::EndpointNotBound
            | NativeProcessAdapterError::SessionNotBound => MobileShellError::SessionNotBound,
            NativeProcessAdapterError::ChildProcessRuntimeDisabled => {
                MobileShellError::ProcessAdapter(error)
            }
        }
    }
}
