//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract

use deve_core::native_adapter::{NativeShellCapabilities, validate_native_endpoint_ready};

use super::DesktopShell;
use crate::types::{
    DesktopBootstrap, DesktopRecoveryBootstrap, DesktopServiceState, DesktopShellError,
    DesktopShellSnapshot,
};

impl DesktopShell {
    pub fn bootstrap_for_web(&mut self) -> Result<DesktopBootstrap, DesktopShellError> {
        self.blocking_state_error()?;
        let endpoint = self
            .core
            .endpoint()
            .ok_or(DesktopShellError::SessionNotBound)?;
        validate_native_endpoint_ready(endpoint)?;
        self.state = DesktopServiceState::WebShellLoading;

        Ok(DesktopBootstrap {
            http_base: endpoint.http_base.clone(),
            ws_base: endpoint.ws_base.clone(),
            node_role: endpoint.node_role.clone(),
            session_bound: endpoint.session_bound,
            capabilities: NativeShellCapabilities::local_backend(),
        })
    }

    pub fn recovery_bootstrap_for_web(&self) -> Option<DesktopRecoveryBootstrap> {
        match self.state {
            DesktopServiceState::ServiceRestarting | DesktopServiceState::ServiceOffline => {
                Some(DesktopRecoveryBootstrap {
                    service_state: "service_offline",
                    capabilities: NativeShellCapabilities::local_backend(),
                })
            }
            DesktopServiceState::SessionInvalid => Some(DesktopRecoveryBootstrap {
                service_state: "session_invalid",
                capabilities: NativeShellCapabilities::local_backend(),
            }),
            DesktopServiceState::ForegroundReprobe => Some(DesktopRecoveryBootstrap {
                service_state: "foreground_reprobe",
                capabilities: NativeShellCapabilities::local_backend(),
            }),
            DesktopServiceState::ColdStart
            | DesktopServiceState::ServiceStarting
            | DesktopServiceState::EndpointBound
            | DesktopServiceState::SessionBound
            | DesktopServiceState::WebShellLoading
            | DesktopServiceState::RuntimeReady => None,
        }
    }

    pub fn snapshot(&self) -> DesktopShellSnapshot {
        let core = self.core.snapshot();
        DesktopShellSnapshot {
            state: self.state.clone(),
            endpoint: core.endpoint,
            readiness: core.readiness,
            offline: core.offline,
            restarting: core.restarting,
            supervisor: core.supervisor,
            process_adapter: core.process_adapter,
        }
    }

    fn blocking_state_error(&self) -> Result<(), DesktopShellError> {
        match self.state {
            DesktopServiceState::ServiceRestarting | DesktopServiceState::ServiceOffline => {
                Err(DesktopShellError::ServiceOffline {
                    reason: self.core.offline_reason_or_unknown(),
                })
            }
            DesktopServiceState::SessionInvalid => Err(DesktopShellError::SessionInvalid),
            DesktopServiceState::ForegroundReprobe => {
                Err(DesktopShellError::ForegroundReprobeRequired)
            }
            _ => Ok(()),
        }
    }
}
