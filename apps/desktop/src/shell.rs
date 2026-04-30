//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract

use deve_core::native_adapter::{
    NativeAdapterPlatform, NativeEndpointReady, NativePlatformEventEffect, NativePlatformEventKind,
    NativeProcessAdapter, NativeProcessAdapterError, NativeProcessAdapterSnapshot,
    NativeRuntimeReadiness, NativeServiceFailureKind, NativeServiceOffline,
    NativeServiceRestarting, NativeServiceSupervisor, NativeServiceSupervisorError,
    NativeServiceSupervisorSnapshot, classify_native_platform_event,
    validate_native_endpoint_ready,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesktopServiceState {
    ColdStart,
    ServiceStarting,
    EndpointBound,
    SessionBound,
    WebShellLoading,
    RuntimeReady,
    ServiceRestarting,
    ServiceOffline,
    SessionInvalid,
    ForegroundReprobe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopShellSnapshot {
    pub state: DesktopServiceState,
    pub endpoint: Option<NativeEndpointReady>,
    pub readiness: NativeRuntimeReadiness,
    pub offline: Option<NativeServiceOffline>,
    pub restarting: Option<NativeServiceRestarting>,
    pub supervisor: NativeServiceSupervisorSnapshot,
    pub process_adapter: NativeProcessAdapterSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopSessionMaterial {
    bound: bool,
}

impl DesktopSessionMaterial {
    pub fn bound() -> Self {
        Self { bound: true }
    }

    pub fn pending() -> Self {
        Self { bound: false }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopBootstrap {
    pub http_base: String,
    pub ws_base: String,
    pub node_role: String,
    pub session_bound: bool,
}

impl DesktopBootstrap {
    pub fn script_tag(&self) -> Result<String, DesktopShellError> {
        let payload = serde_json::to_string(self)?;
        Ok(format!(
            "<script>window.__DEVE_NATIVE_BOOTSTRAP={payload};</script>"
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DesktopRecoveryBootstrap {
    pub service_state: &'static str,
}

impl DesktopRecoveryBootstrap {
    pub fn script_tag(&self) -> Result<String, DesktopShellError> {
        let payload = serde_json::to_string(self)?;
        Ok(format!(
            "<script>window.__DEVE_NATIVE_BOOTSTRAP={payload};</script>"
        ))
    }
}

#[derive(Debug, Error)]
pub enum DesktopShellError {
    #[error("desktop service endpoint is invalid: {0}")]
    InvalidEndpoint(#[from] deve_core::native_adapter::NativeAdapterError),
    #[error("desktop session is not bound")]
    SessionNotBound,
    #[error("desktop service is offline: {reason}")]
    ServiceOffline { reason: String },
    #[error("desktop session is invalid")]
    SessionInvalid,
    #[error("desktop foreground reprobe is required before loading writable shell")]
    ForegroundReprobeRequired,
    #[error("desktop service supervisor rejected transition: {0}")]
    Supervisor(#[from] NativeServiceSupervisorError),
    #[error("desktop process adapter rejected transition: {0}")]
    ProcessAdapter(#[from] NativeProcessAdapterError),
    #[error("failed to serialize desktop bootstrap: {0}")]
    BootstrapSerialize(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct DesktopShell {
    state: DesktopServiceState,
    endpoint: Option<NativeEndpointReady>,
    readiness: NativeRuntimeReadiness,
    offline: Option<NativeServiceOffline>,
    restarting: Option<NativeServiceRestarting>,
    supervisor: NativeServiceSupervisor,
    process_adapter: NativeProcessAdapter,
}

impl Default for DesktopShell {
    fn default() -> Self {
        Self::new()
    }
}

impl DesktopShell {
    pub fn new() -> Self {
        Self {
            state: DesktopServiceState::ColdStart,
            endpoint: None,
            readiness: NativeRuntimeReadiness::default(),
            offline: None,
            restarting: None,
            supervisor: NativeServiceSupervisor::new(2),
            process_adapter: NativeProcessAdapter::default(),
        }
    }

    pub fn start_service(&mut self) {
        self.state = DesktopServiceState::ServiceStarting;
        self.offline = None;
        self.restarting = None;
        self.supervisor.start();
    }

    pub fn bind_endpoint(
        &mut self,
        endpoint: NativeEndpointReady,
    ) -> Result<(), DesktopShellError> {
        let process_snapshot = self
            .process_adapter
            .bind_existing_endpoint(endpoint)
            .map_err(Self::map_process_adapter_error)?;
        self.supervisor
            .record_health_probe(process_snapshot.health_probe)?;
        self.readiness.endpoint_reachable = process_snapshot.health_probe.endpoint_reachable;
        self.readiness.node_role_readable = process_snapshot.health_probe.node_role_readable;
        self.endpoint = process_snapshot.endpoint;
        self.state = DesktopServiceState::EndpointBound;
        self.offline = None;
        self.restarting = None;
        Ok(())
    }

    pub fn bind_session(
        &mut self,
        session: DesktopSessionMaterial,
    ) -> Result<(), DesktopShellError> {
        if !session.bound {
            return Err(DesktopShellError::SessionNotBound);
        }
        let endpoint = self
            .endpoint
            .as_mut()
            .ok_or(DesktopShellError::SessionNotBound)?;
        self.supervisor.record_session_handoff(session.bound)?;
        let process_snapshot = self
            .process_adapter
            .bind_session(session.bound)
            .map_err(Self::map_process_adapter_error)?;
        let process_endpoint = process_snapshot
            .endpoint
            .ok_or(DesktopShellError::SessionNotBound)?;
        *endpoint = process_endpoint;
        validate_native_endpoint_ready(endpoint)?;
        self.readiness.auth_status_valid = true;
        self.state = DesktopServiceState::SessionBound;
        Ok(())
    }

    pub fn mark_runtime_ready(&mut self, readiness: NativeRuntimeReadiness) -> bool {
        let ready = readiness.is_runtime_ready();
        self.readiness = readiness;
        if ready {
            self.state = DesktopServiceState::RuntimeReady;
            self.offline = None;
            self.restarting = None;
        }
        ready
    }

    pub fn bootstrap_for_web(&mut self) -> Result<DesktopBootstrap, DesktopShellError> {
        match &self.state {
            DesktopServiceState::ServiceRestarting | DesktopServiceState::ServiceOffline => {
                let reason = self
                    .offline
                    .as_ref()
                    .map(|offline| offline.reason.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                return Err(DesktopShellError::ServiceOffline { reason });
            }
            DesktopServiceState::SessionInvalid => return Err(DesktopShellError::SessionInvalid),
            DesktopServiceState::ForegroundReprobe => {
                return Err(DesktopShellError::ForegroundReprobeRequired);
            }
            _ => {}
        }

        let endpoint = self
            .endpoint
            .as_ref()
            .ok_or(DesktopShellError::SessionNotBound)?;
        validate_native_endpoint_ready(endpoint)?;
        self.state = DesktopServiceState::WebShellLoading;

        Ok(DesktopBootstrap {
            http_base: endpoint.http_base.clone(),
            ws_base: endpoint.ws_base.clone(),
            node_role: endpoint.node_role.clone(),
            session_bound: endpoint.session_bound,
        })
    }

    pub fn recovery_bootstrap_for_web(&self) -> Option<DesktopRecoveryBootstrap> {
        match self.state {
            DesktopServiceState::ServiceRestarting | DesktopServiceState::ServiceOffline => {
                Some(DesktopRecoveryBootstrap {
                    service_state: "service_offline",
                })
            }
            DesktopServiceState::SessionInvalid => Some(DesktopRecoveryBootstrap {
                service_state: "session_invalid",
            }),
            DesktopServiceState::ForegroundReprobe => Some(DesktopRecoveryBootstrap {
                service_state: "foreground_reprobe",
            }),
            DesktopServiceState::ColdStart
            | DesktopServiceState::ServiceStarting
            | DesktopServiceState::EndpointBound
            | DesktopServiceState::SessionBound
            | DesktopServiceState::WebShellLoading
            | DesktopServiceState::RuntimeReady => None,
        }
    }

    pub fn handle_platform_event(
        &mut self,
        event: NativePlatformEventKind,
    ) -> NativePlatformEventEffect {
        let effect = classify_native_platform_event(NativeAdapterPlatform::Desktop, event);
        if effect == NativePlatformEventEffect::RequireForegroundReprobe
            && self.is_service_recovery_state()
        {
            return NativePlatformEventEffect::NoBusinessStateChange;
        }
        if effect == NativePlatformEventEffect::RequireForegroundReprobe {
            self.require_foreground_reprobe();
        }
        effect
    }

    pub fn complete_foreground_reprobe(&mut self, readiness: NativeRuntimeReadiness) -> bool {
        if self.state != DesktopServiceState::ForegroundReprobe {
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
        self.state = DesktopServiceState::SessionInvalid;
        self.readiness.auth_status_valid = false;
        self.process_adapter.clear_session();
        if let Some(endpoint) = self.endpoint.as_mut() {
            endpoint.session_bound = false;
        }
    }

    pub fn snapshot(&self) -> DesktopShellSnapshot {
        DesktopShellSnapshot {
            state: self.state.clone(),
            endpoint: self.endpoint.clone(),
            readiness: self.readiness,
            offline: self.offline.clone(),
            restarting: self.restarting.clone(),
            supervisor: self.supervisor.snapshot(),
            process_adapter: self.process_adapter.snapshot(),
        }
    }

    fn require_foreground_reprobe(&mut self) {
        self.state = DesktopServiceState::ForegroundReprobe;
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

    fn is_service_recovery_state(&self) -> bool {
        matches!(
            self.state,
            DesktopServiceState::ServiceRestarting | DesktopServiceState::ServiceOffline
        )
    }

    fn record_offline_snapshot(&mut self, offline: NativeServiceOffline) {
        let supervisor = self.supervisor.snapshot();
        self.restarting = offline.retryable.then_some(NativeServiceRestarting {
            attempt: supervisor.restart_attempt,
        });
        self.state = if offline.retryable {
            DesktopServiceState::ServiceRestarting
        } else {
            DesktopServiceState::ServiceOffline
        };
        self.offline = Some(offline);
    }

    fn map_process_adapter_error(error: NativeProcessAdapterError) -> DesktopShellError {
        match error {
            NativeProcessAdapterError::InvalidEndpoint(error) => {
                DesktopShellError::InvalidEndpoint(error)
            }
            NativeProcessAdapterError::EndpointNotBound
            | NativeProcessAdapterError::SessionNotBound => DesktopShellError::SessionNotBound,
            NativeProcessAdapterError::ChildProcessRuntimeDisabled => {
                DesktopShellError::ProcessAdapter(error)
            }
        }
    }
}
