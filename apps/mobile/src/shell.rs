//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract

use deve_core::native_adapter::{
    NativeEndpointReady, NativePlatformEventKind, NativeProcessAdapter, NativeProcessAdapterError,
    NativeProcessAdapterSnapshot, NativeRuntimeReadiness, NativeServiceFailureKind,
    NativeServiceOffline, NativeServiceRestarting, NativeServiceSupervisor,
    NativeServiceSupervisorError, NativeServiceSupervisorObservation, NativeServiceSupervisorState,
    NativeServiceSuspended, validate_native_endpoint_ready,
};

use crate::types::{
    MobileBootstrap, MobileLifecycleEvent, MobileRecoveryBootstrap, MobileServiceState,
    MobileSessionMaterial, MobileShellError, MobileShellSnapshot,
};

#[derive(Debug, Clone)]
pub struct MobileShell {
    state: MobileServiceState,
    endpoint: Option<NativeEndpointReady>,
    readiness: NativeRuntimeReadiness,
    offline: Option<NativeServiceOffline>,
    restarting: Option<NativeServiceRestarting>,
    suspended: Option<NativeServiceSuspended>,
    supervisor: NativeServiceSupervisor,
    process_adapter: NativeProcessAdapter,
}

impl Default for MobileShell {
    fn default() -> Self {
        Self::new()
    }
}

impl MobileShell {
    pub fn new() -> Self {
        Self {
            state: MobileServiceState::ColdStart,
            endpoint: None,
            readiness: NativeRuntimeReadiness::default(),
            offline: None,
            restarting: None,
            suspended: None,
            supervisor: NativeServiceSupervisor::new(2),
            process_adapter: NativeProcessAdapter::default(),
        }
    }

    pub fn start_service(&mut self) {
        self.state = MobileServiceState::ServiceStarting;
        self.offline = None;
        self.restarting = None;
        self.suspended = None;
        self.supervisor.start();
    }

    pub fn bind_endpoint(&mut self, endpoint: NativeEndpointReady) -> Result<(), MobileShellError> {
        self.ensure_not_terminal_offline()?;
        let process_snapshot = self
            .process_adapter
            .bind_existing_endpoint(endpoint)
            .map_err(Self::map_process_adapter_error)?;
        self.observe_process_snapshot(&process_snapshot)?;
        self.readiness.endpoint_reachable = process_snapshot.health_probe.endpoint_reachable;
        self.readiness.node_role_readable = process_snapshot.health_probe.node_role_readable;
        self.endpoint = process_snapshot.endpoint;
        self.state = MobileServiceState::EndpointBound;
        self.offline = None;
        self.restarting = None;
        Ok(())
    }

    pub fn bind_session(&mut self, session: MobileSessionMaterial) -> Result<(), MobileShellError> {
        if !session.bound {
            return Err(MobileShellError::SessionNotBound);
        }
        self.endpoint
            .as_ref()
            .ok_or(MobileShellError::SessionNotBound)?;
        let process_snapshot = self
            .process_adapter
            .bind_session(session.bound)
            .map_err(Self::map_process_adapter_error)?;
        self.observe_process_snapshot(&process_snapshot)?;
        let process_endpoint = process_snapshot
            .endpoint
            .ok_or(MobileShellError::SessionNotBound)?;
        validate_native_endpoint_ready(&process_endpoint)?;
        self.endpoint = Some(process_endpoint);
        self.readiness.auth_status_valid = true;
        self.readiness.node_role_readable = true;
        self.state = MobileServiceState::SessionBound;
        Ok(())
    }

    pub fn mark_probe_timeout(&mut self) -> Result<(), MobileShellError> {
        let process_snapshot = self.process_adapter.record_probe_timeout();
        let result = self.observe_process_snapshot(&process_snapshot);
        self.clear_shell_runtime_binding();
        result
    }

    pub fn mark_process_shutdown(&mut self) -> Result<(), MobileShellError> {
        let process_snapshot = self.process_adapter.record_process_stopped();
        let result = self.observe_process_snapshot(&process_snapshot);
        self.clear_shell_runtime_binding();
        result
    }

    pub fn mark_runtime_ready(&mut self, readiness: NativeRuntimeReadiness) -> bool {
        if self.terminal_offline_reason().is_some() {
            return false;
        }
        let ready = readiness.is_runtime_ready();
        self.readiness = readiness;
        if ready {
            self.state = MobileServiceState::RuntimeReady;
            self.offline = None;
            self.restarting = None;
            self.suspended = None;
        }
        ready
    }

    pub fn bootstrap_for_web(&mut self) -> Result<MobileBootstrap, MobileShellError> {
        self.blocking_state_error()?;
        let endpoint = self
            .endpoint
            .as_ref()
            .ok_or(MobileShellError::SessionNotBound)?;
        validate_native_endpoint_ready(endpoint)?;
        self.state = MobileServiceState::WebShellLoading;
        Ok(MobileBootstrap {
            http_base: endpoint.http_base.clone(),
            ws_base: endpoint.ws_base.clone(),
            node_role: endpoint.node_role.clone(),
            session_bound: endpoint.session_bound,
        })
    }

    pub fn recovery_bootstrap_for_web(&self) -> Option<MobileRecoveryBootstrap> {
        match self.state {
            MobileServiceState::ServiceRestarting | MobileServiceState::ServiceOffline => {
                Some(MobileRecoveryBootstrap {
                    service_state: "service_offline",
                })
            }
            MobileServiceState::SessionInvalid => Some(MobileRecoveryBootstrap {
                service_state: "session_invalid",
            }),
            MobileServiceState::ForegroundReprobe | MobileServiceState::BackgroundSuspended => {
                Some(MobileRecoveryBootstrap {
                    service_state: "foreground_reprobe",
                })
            }
            MobileServiceState::ColdStart
            | MobileServiceState::ServiceStarting
            | MobileServiceState::EndpointBound
            | MobileServiceState::SessionBound
            | MobileServiceState::WebShellLoading
            | MobileServiceState::RuntimeReady => None,
        }
    }

    pub fn handle_lifecycle_event(
        &mut self,
        event: MobileLifecycleEvent,
    ) -> NativePlatformEventKind {
        let kind = event.to_native_kind();
        if self.is_service_recovery_state() {
            return kind;
        }
        match event {
            MobileLifecycleEvent::Background | MobileLifecycleEvent::Suspended => {
                self.state = MobileServiceState::BackgroundSuspended;
                self.suspended = Some(NativeServiceSuspended {
                    reason: format!("{kind:?}"),
                });
            }
            MobileLifecycleEvent::Foreground | MobileLifecycleEvent::Resumed => {
                self.require_foreground_reprobe();
            }
            MobileLifecycleEvent::NetworkOnline
            | MobileLifecycleEvent::NetworkOffline
            | MobileLifecycleEvent::SafeAreaChanged
            | MobileLifecycleEvent::KeyboardChanged => {}
        }
        kind
    }

    pub fn complete_foreground_reprobe(&mut self, readiness: NativeRuntimeReadiness) -> bool {
        if self.state != MobileServiceState::ForegroundReprobe {
            return false;
        }
        self.mark_runtime_ready(readiness)
    }

    pub fn mark_service_offline(&mut self, reason: impl Into<String>, retryable: bool) {
        let offline = NativeServiceOffline {
            reason: reason.into(),
            retryable,
        };
        let offline = self.supervisor.record_service_offline(offline);
        self.record_offline_snapshot(offline);
        self.clear_runtime_binding();
    }

    pub fn mark_supervisor_failure(
        &mut self,
        kind: NativeServiceFailureKind,
        reason: impl Into<String>,
    ) {
        let offline = self.supervisor.record_failure(kind, reason);
        self.record_offline_snapshot(offline);
        self.clear_runtime_binding();
    }

    pub fn invalidate_session(&mut self) {
        self.state = MobileServiceState::SessionInvalid;
        self.readiness.auth_status_valid = false;
        self.process_adapter.clear_session();
        if let Some(endpoint) = self.endpoint.as_mut() {
            endpoint.session_bound = false;
        }
    }

    pub fn snapshot(&self) -> MobileShellSnapshot {
        MobileShellSnapshot {
            state: self.state.clone(),
            endpoint: self.endpoint.clone(),
            readiness: self.readiness,
            offline: self.offline.clone(),
            restarting: self.restarting.clone(),
            suspended: self.suspended.clone(),
            supervisor: self.supervisor.snapshot(),
            process_adapter: self.process_adapter.snapshot(),
        }
    }

    fn require_foreground_reprobe(&mut self) {
        self.state = MobileServiceState::ForegroundReprobe;
        self.suspended = None;
        self.readiness.auth_status_valid = false;
        self.readiness.node_role_readable = false;
        self.readiness.repo_handshake_complete = false;
        self.readiness.writer_ready = false;
        self.readiness.scope_nonce_current = false;
    }

    fn clear_runtime_binding(&mut self) {
        self.readiness = NativeRuntimeReadiness::default();
        self.process_adapter.record_process_stopped();
        self.endpoint = None;
    }

    fn clear_shell_runtime_binding(&mut self) {
        self.readiness = NativeRuntimeReadiness::default();
        self.endpoint = None;
    }

    fn is_service_recovery_state(&self) -> bool {
        matches!(
            self.state,
            MobileServiceState::ServiceRestarting | MobileServiceState::ServiceOffline
        )
    }

    fn ensure_not_terminal_offline(&self) -> Result<(), MobileShellError> {
        let Some(reason) = self.terminal_offline_reason() else {
            return Ok(());
        };
        Err(MobileShellError::ServiceOffline { reason })
    }

    fn terminal_offline_reason(&self) -> Option<String> {
        let supervisor = self.supervisor.snapshot();
        if supervisor.state != NativeServiceSupervisorState::Offline {
            return None;
        }
        supervisor
            .offline
            .filter(|offline| !offline.retryable)
            .map(|offline| offline.reason)
    }

    fn blocking_state_error(&self) -> Result<(), MobileShellError> {
        match self.state {
            MobileServiceState::ServiceRestarting | MobileServiceState::ServiceOffline => {
                Err(MobileShellError::ServiceOffline {
                    reason: self
                        .offline
                        .as_ref()
                        .map(|offline| offline.reason.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                })
            }
            MobileServiceState::SessionInvalid => Err(MobileShellError::SessionInvalid),
            MobileServiceState::ForegroundReprobe | MobileServiceState::BackgroundSuspended => {
                Err(MobileShellError::ForegroundReprobeRequired)
            }
            _ => Ok(()),
        }
    }

    fn map_process_adapter_error(error: NativeProcessAdapterError) -> MobileShellError {
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

    fn record_offline_snapshot(&mut self, offline: NativeServiceOffline) {
        let supervisor = self.supervisor.snapshot();
        self.restarting = offline.retryable.then_some(NativeServiceRestarting {
            attempt: supervisor.restart_attempt,
        });
        self.state = if offline.retryable {
            MobileServiceState::ServiceRestarting
        } else {
            MobileServiceState::ServiceOffline
        };
        self.offline = Some(offline);
        self.suspended = None;
    }

    fn observe_process_snapshot(
        &mut self,
        process_snapshot: &NativeProcessAdapterSnapshot,
    ) -> Result<(), MobileShellError> {
        let observation = self.supervisor.record_process_snapshot(process_snapshot);
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
                self.record_offline_snapshot(offline);
                Err(MobileShellError::ServiceOffline { reason })
            }
            NativeServiceSupervisorObservation::Idle => Err(MobileShellError::Supervisor(
                NativeServiceSupervisorError::EndpointNotHealthy,
            )),
        }
    }
}
