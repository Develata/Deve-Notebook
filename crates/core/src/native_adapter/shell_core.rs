//! plan_ref:
//!   - 11_ui_design_02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design_03_mobile#mobile-native-adapter-contract
//!   - 11_ui_design#native-adapter-gate-registry
//!
use serde::{Deserialize, Serialize};

use super::{
    NativeEndpointReady, NativeProcessAdapter, NativeProcessAdapterError,
    NativeProcessAdapterSnapshot, NativeRuntimeReadiness, NativeServiceFailureKind,
    NativeServiceOffline, NativeServiceRestarting, NativeServiceSupervisor,
    NativeServiceSupervisorObservation, NativeServiceSupervisorSnapshot,
    NativeServiceSupervisorState, validate_native_endpoint_ready,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeShellCoreSnapshot {
    pub endpoint: Option<NativeEndpointReady>,
    pub readiness: NativeRuntimeReadiness,
    pub offline: Option<NativeServiceOffline>,
    pub restarting: Option<NativeServiceRestarting>,
    pub supervisor: NativeServiceSupervisorSnapshot,
    pub process_adapter: NativeProcessAdapterSnapshot,
}

#[derive(Debug, Clone)]
pub struct NativeShellCore {
    endpoint: Option<NativeEndpointReady>,
    readiness: NativeRuntimeReadiness,
    offline: Option<NativeServiceOffline>,
    restarting: Option<NativeServiceRestarting>,
    supervisor: NativeServiceSupervisor,
    process_adapter: NativeProcessAdapter,
}

impl NativeShellCore {
    pub fn new(max_restart_attempts: u32) -> Self {
        Self {
            endpoint: None,
            readiness: NativeRuntimeReadiness::default(),
            offline: None,
            restarting: None,
            supervisor: NativeServiceSupervisor::new(max_restart_attempts),
            process_adapter: NativeProcessAdapter::default(),
        }
    }

    pub fn start_service(&mut self) -> bool {
        if self.terminal_offline_reason().is_some() {
            return false;
        }
        self.offline = None;
        self.restarting = None;
        self.supervisor.start();
        true
    }

    pub fn bind_existing_endpoint(
        &mut self,
        endpoint: NativeEndpointReady,
    ) -> Result<NativeProcessAdapterSnapshot, NativeProcessAdapterError> {
        self.process_adapter.bind_existing_endpoint(endpoint)
    }

    pub fn bind_session(
        &mut self,
        session_bound: bool,
    ) -> Result<NativeProcessAdapterSnapshot, NativeProcessAdapterError> {
        self.process_adapter.bind_session(session_bound)
    }

    pub fn record_probe_timeout(&mut self) -> NativeProcessAdapterSnapshot {
        self.process_adapter.record_probe_timeout()
    }

    pub fn record_process_stopped(&mut self) -> NativeProcessAdapterSnapshot {
        self.process_adapter.record_process_stopped()
    }

    pub fn record_service_offline(
        &mut self,
        reason: impl Into<String>,
        retryable: bool,
    ) -> NativeServiceOffline {
        let offline = self
            .supervisor
            .record_service_offline(NativeServiceOffline {
                reason: reason.into(),
                retryable,
            });
        self.record_offline_snapshot(offline.clone());
        offline
    }

    pub fn record_failure(
        &mut self,
        kind: NativeServiceFailureKind,
        reason: impl Into<String>,
    ) -> NativeServiceOffline {
        let offline = self.supervisor.record_failure(kind, reason);
        self.record_offline_snapshot(offline.clone());
        offline
    }

    pub fn observe_process_snapshot(
        &mut self,
        process_snapshot: &NativeProcessAdapterSnapshot,
    ) -> NativeServiceSupervisorObservation {
        let observation = self.supervisor.record_process_snapshot(process_snapshot);
        if let NativeServiceSupervisorObservation::Offline(offline) = &observation {
            self.record_offline_snapshot(offline.clone());
        }
        observation
    }

    pub fn apply_endpoint_probe_snapshot(
        &mut self,
        process_snapshot: &NativeProcessAdapterSnapshot,
    ) {
        self.readiness.endpoint_reachable = process_snapshot.health_probe.endpoint_reachable;
        self.readiness.node_role_readable = process_snapshot.health_probe.node_role_readable;
        self.endpoint = process_snapshot.endpoint.clone();
        self.offline = None;
        self.restarting = None;
    }

    pub fn apply_session_snapshot(
        &mut self,
        process_snapshot: &NativeProcessAdapterSnapshot,
    ) -> Result<(), super::NativeAdapterError> {
        let process_endpoint = process_snapshot
            .endpoint
            .clone()
            .ok_or(super::NativeAdapterError::SessionNotBound)?;
        validate_native_endpoint_ready(&process_endpoint)?;
        self.endpoint = Some(process_endpoint);
        self.readiness.auth_status_valid = true;
        Ok(())
    }

    pub fn set_runtime_readiness(&mut self, readiness: NativeRuntimeReadiness) -> bool {
        let ready = readiness.is_runtime_ready();
        self.readiness = readiness;
        if ready {
            self.offline = None;
            self.restarting = None;
        }
        ready
    }

    pub fn require_foreground_reprobe(&mut self) {
        self.readiness.auth_status_valid = false;
        self.readiness.node_role_readable = false;
        self.readiness.repo_handshake_complete = false;
        self.readiness.writer_ready = false;
        self.readiness.scope_nonce_current = false;
    }

    pub fn clear_runtime_binding(&mut self) {
        self.readiness = NativeRuntimeReadiness::default();
        self.process_adapter.record_process_stopped();
        self.endpoint = None;
    }

    pub fn clear_shell_runtime_binding(&mut self) {
        self.readiness = NativeRuntimeReadiness::default();
        self.endpoint = None;
    }

    pub fn invalidate_session_binding(&mut self) {
        self.readiness.auth_status_valid = false;
        self.process_adapter.clear_session();
        if let Some(endpoint) = self.endpoint.as_mut() {
            endpoint.session_bound = false;
        }
    }

    pub fn endpoint(&self) -> Option<&NativeEndpointReady> {
        self.endpoint.as_ref()
    }

    pub fn mark_node_role_readable(&mut self) {
        self.readiness.node_role_readable = true;
    }

    pub fn offline_reason_or_unknown(&self) -> String {
        self.offline
            .as_ref()
            .map(|offline| offline.reason.clone())
            .unwrap_or_else(|| "unknown".to_string())
    }

    pub fn terminal_offline_reason(&self) -> Option<String> {
        let supervisor = self.supervisor.snapshot();
        if supervisor.state != NativeServiceSupervisorState::Offline {
            return None;
        }
        supervisor
            .offline
            .filter(|offline| !offline.retryable)
            .map(|offline| offline.reason)
    }

    pub fn snapshot(&self) -> NativeShellCoreSnapshot {
        NativeShellCoreSnapshot {
            endpoint: self.endpoint.clone(),
            readiness: self.readiness,
            offline: self.offline.clone(),
            restarting: self.restarting.clone(),
            supervisor: self.supervisor.snapshot(),
            process_adapter: self.process_adapter.snapshot(),
        }
    }

    fn record_offline_snapshot(&mut self, offline: NativeServiceOffline) {
        let supervisor = self.supervisor.snapshot();
        self.restarting = offline.retryable.then_some(NativeServiceRestarting {
            attempt: supervisor.restart_attempt,
        });
        self.offline = Some(offline);
    }
}
