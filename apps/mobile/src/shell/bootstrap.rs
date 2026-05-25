//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract

use deve_core::native_adapter::validate_native_endpoint_ready;

use super::MobileShell;
use crate::types::{
    MobileBootstrap, MobileRecoveryBootstrap, MobileServiceState, MobileShellError,
    MobileShellSnapshot,
};

impl MobileShell {
    pub fn bootstrap_for_web(&mut self) -> Result<MobileBootstrap, MobileShellError> {
        self.blocking_state_error()?;
        let endpoint = self
            .core
            .endpoint()
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

    pub fn snapshot(&self) -> MobileShellSnapshot {
        let core = self.core.snapshot();
        MobileShellSnapshot {
            state: self.state.clone(),
            endpoint: core.endpoint,
            readiness: core.readiness,
            offline: core.offline,
            restarting: core.restarting,
            suspended: self.suspended.clone(),
            supervisor: core.supervisor,
            process_adapter: core.process_adapter,
        }
    }

    fn blocking_state_error(&self) -> Result<(), MobileShellError> {
        match self.state {
            MobileServiceState::ServiceRestarting | MobileServiceState::ServiceOffline => {
                Err(MobileShellError::ServiceOffline {
                    reason: self.core.offline_reason_or_unknown(),
                })
            }
            MobileServiceState::SessionInvalid => Err(MobileShellError::SessionInvalid),
            MobileServiceState::ForegroundReprobe | MobileServiceState::BackgroundSuspended => {
                Err(MobileShellError::ForegroundReprobeRequired)
            }
            _ => Ok(()),
        }
    }
}
