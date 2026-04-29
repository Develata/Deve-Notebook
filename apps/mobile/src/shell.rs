//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract

use deve_core::native_adapter::{
    NativeEndpointReady, NativePlatformEventKind, NativeRuntimeReadiness, NativeServiceFailureKind,
    NativeServiceHealthProbe, NativeServiceOffline, NativeServiceSupervisor,
    NativeServiceSuspended, validate_native_endpoint_bases, validate_native_endpoint_ready,
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
    suspended: Option<NativeServiceSuspended>,
    supervisor: NativeServiceSupervisor,
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
            suspended: None,
            supervisor: NativeServiceSupervisor::new(2),
        }
    }

    pub fn start_service(&mut self) {
        self.state = MobileServiceState::ServiceStarting;
        self.offline = None;
        self.suspended = None;
        self.supervisor.start();
    }

    pub fn bind_endpoint(
        &mut self,
        mut endpoint: NativeEndpointReady,
    ) -> Result<(), MobileShellError> {
        endpoint.session_bound = false;
        validate_native_endpoint_bases(&endpoint)?;
        self.supervisor
            .record_health_probe(NativeServiceHealthProbe {
                endpoint_reachable: true,
                node_role_readable: true,
            })?;
        self.readiness.endpoint_reachable = true;
        self.endpoint = Some(endpoint);
        self.state = MobileServiceState::EndpointBound;
        Ok(())
    }

    pub fn bind_session(&mut self, session: MobileSessionMaterial) -> Result<(), MobileShellError> {
        if !session.bound {
            return Err(MobileShellError::SessionNotBound);
        }
        let endpoint = self
            .endpoint
            .as_mut()
            .ok_or(MobileShellError::SessionNotBound)?;
        self.supervisor.record_session_handoff(session.bound)?;
        endpoint.session_bound = true;
        validate_native_endpoint_ready(endpoint)?;
        self.readiness.auth_status_valid = true;
        self.readiness.node_role_readable = true;
        self.state = MobileServiceState::SessionBound;
        Ok(())
    }

    pub fn mark_runtime_ready(&mut self, readiness: NativeRuntimeReadiness) -> bool {
        let ready = readiness.is_runtime_ready();
        self.readiness = readiness;
        if ready {
            self.state = MobileServiceState::RuntimeReady;
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
            MobileServiceState::ServiceOffline => Some(MobileRecoveryBootstrap {
                service_state: "service_offline",
            }),
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
        self.state = MobileServiceState::ServiceOffline;
        self.offline = Some(NativeServiceOffline {
            reason: reason.into(),
            retryable,
        });
        self.readiness.endpoint_reachable = false;
    }

    pub fn mark_supervisor_failure(
        &mut self,
        kind: NativeServiceFailureKind,
        reason: impl Into<String>,
    ) {
        let offline = self.supervisor.record_failure(kind, reason);
        self.state = MobileServiceState::ServiceOffline;
        self.offline = Some(offline);
        self.readiness.endpoint_reachable = false;
    }

    pub fn invalidate_session(&mut self) {
        self.state = MobileServiceState::SessionInvalid;
        self.readiness.auth_status_valid = false;
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
            suspended: self.suspended.clone(),
            supervisor: self.supervisor.snapshot(),
        }
    }

    fn require_foreground_reprobe(&mut self) {
        self.state = MobileServiceState::ForegroundReprobe;
        self.readiness.auth_status_valid = false;
        self.readiness.repo_handshake_complete = false;
        self.readiness.writer_ready = false;
        self.readiness.scope_nonce_current = false;
    }

    fn blocking_state_error(&self) -> Result<(), MobileShellError> {
        match self.state {
            MobileServiceState::ServiceOffline => Err(MobileShellError::ServiceOffline {
                reason: self
                    .offline
                    .as_ref()
                    .map(|offline| offline.reason.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
            }),
            MobileServiceState::SessionInvalid => Err(MobileShellError::SessionInvalid),
            MobileServiceState::ForegroundReprobe | MobileServiceState::BackgroundSuspended => {
                Err(MobileShellError::ForegroundReprobeRequired)
            }
            _ => Ok(()),
        }
    }
}
