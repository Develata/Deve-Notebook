//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract

use deve_core::native_adapter::{
    NativeEndpointReady, NativeServiceOffline, validate_native_endpoint_bases,
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
    ServiceOffline,
    SessionInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopShellSnapshot {
    pub state: DesktopServiceState,
    pub endpoint: Option<NativeEndpointReady>,
    pub offline: Option<NativeServiceOffline>,
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
    #[error("failed to serialize desktop bootstrap: {0}")]
    BootstrapSerialize(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct DesktopShell {
    state: DesktopServiceState,
    endpoint: Option<NativeEndpointReady>,
    offline: Option<NativeServiceOffline>,
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
            offline: None,
        }
    }

    pub fn start_service(&mut self) {
        self.state = DesktopServiceState::ServiceStarting;
        self.offline = None;
    }

    pub fn bind_endpoint(
        &mut self,
        mut endpoint: NativeEndpointReady,
    ) -> Result<(), DesktopShellError> {
        endpoint.session_bound = false;
        validate_native_endpoint_bases(&endpoint)?;
        self.endpoint = Some(endpoint);
        self.state = DesktopServiceState::EndpointBound;
        self.offline = None;
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
        endpoint.session_bound = true;
        validate_native_endpoint_ready(endpoint)?;
        self.state = DesktopServiceState::SessionBound;
        Ok(())
    }

    pub fn bootstrap_for_web(&mut self) -> Result<DesktopBootstrap, DesktopShellError> {
        match &self.state {
            DesktopServiceState::ServiceOffline => {
                let reason = self
                    .offline
                    .as_ref()
                    .map(|offline| offline.reason.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                return Err(DesktopShellError::ServiceOffline { reason });
            }
            DesktopServiceState::SessionInvalid => return Err(DesktopShellError::SessionInvalid),
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
            DesktopServiceState::ServiceOffline => Some(DesktopRecoveryBootstrap {
                service_state: "service_offline",
            }),
            DesktopServiceState::SessionInvalid => Some(DesktopRecoveryBootstrap {
                service_state: "session_invalid",
            }),
            DesktopServiceState::ColdStart
            | DesktopServiceState::ServiceStarting
            | DesktopServiceState::EndpointBound
            | DesktopServiceState::SessionBound
            | DesktopServiceState::WebShellLoading => None,
        }
    }

    pub fn mark_service_offline(&mut self, reason: impl Into<String>, retryable: bool) {
        self.state = DesktopServiceState::ServiceOffline;
        self.offline = Some(NativeServiceOffline {
            reason: reason.into(),
            retryable,
        });
    }

    pub fn invalidate_session(&mut self) {
        self.state = DesktopServiceState::SessionInvalid;
        if let Some(endpoint) = self.endpoint.as_mut() {
            endpoint.session_bound = false;
        }
    }

    pub fn snapshot(&self) -> DesktopShellSnapshot {
        DesktopShellSnapshot {
            state: self.state.clone(),
            endpoint: self.endpoint.clone(),
            offline: self.offline.clone(),
        }
    }
}
