//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract

use deve_core::native_adapter::{NativeEndpointReady, NativeServiceSuspended, NativeShellCore};

use crate::types::{MobileServiceState, MobileSessionMaterial, MobileShellError};

mod bootstrap;
mod lifecycle;
mod recovery;

#[derive(Debug, Clone)]
pub struct MobileShell {
    state: MobileServiceState,
    suspended: Option<NativeServiceSuspended>,
    core: NativeShellCore,
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
            suspended: None,
            core: NativeShellCore::new(2),
        }
    }

    pub fn start_service(&mut self) {
        if !self.core.start_service() {
            return;
        }
        self.state = MobileServiceState::ServiceStarting;
        self.suspended = None;
    }

    pub fn bind_endpoint(&mut self, endpoint: NativeEndpointReady) -> Result<(), MobileShellError> {
        self.ensure_not_terminal_offline()?;
        let process_snapshot = self
            .core
            .bind_existing_endpoint(endpoint)
            .map_err(Self::map_process_adapter_error)?;
        self.observe_process_snapshot(&process_snapshot)?;
        self.core.apply_endpoint_probe_snapshot(&process_snapshot);
        self.state = MobileServiceState::EndpointBound;
        Ok(())
    }

    pub fn bind_session(&mut self, session: MobileSessionMaterial) -> Result<(), MobileShellError> {
        if !session.bound {
            return Err(MobileShellError::SessionNotBound);
        }
        self.core
            .endpoint()
            .ok_or(MobileShellError::SessionNotBound)?;
        let process_snapshot = self
            .core
            .bind_session(session.bound)
            .map_err(Self::map_process_adapter_error)?;
        self.observe_process_snapshot(&process_snapshot)?;
        self.core.apply_session_snapshot(&process_snapshot)?;
        self.core.mark_node_role_readable();
        self.state = MobileServiceState::SessionBound;
        Ok(())
    }
}
