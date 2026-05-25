//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract

use deve_core::native_adapter::{NativeEndpointReady, NativeShellCore};

use crate::types::{DesktopServiceState, DesktopSessionMaterial, DesktopShellError};

mod bootstrap;
mod lifecycle;
mod recovery;

#[derive(Debug, Clone)]
pub struct DesktopShell {
    state: DesktopServiceState,
    core: NativeShellCore,
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
            core: NativeShellCore::new(2),
        }
    }

    pub fn start_service(&mut self) {
        if !self.core.start_service() {
            return;
        }
        self.state = DesktopServiceState::ServiceStarting;
    }

    pub fn bind_endpoint(
        &mut self,
        endpoint: NativeEndpointReady,
    ) -> Result<(), DesktopShellError> {
        self.ensure_not_terminal_offline()?;
        let process_snapshot = self
            .core
            .bind_existing_endpoint(endpoint)
            .map_err(Self::map_process_adapter_error)?;
        self.observe_process_snapshot(&process_snapshot)?;
        self.core.apply_endpoint_probe_snapshot(&process_snapshot);
        self.state = DesktopServiceState::EndpointBound;
        Ok(())
    }

    pub fn bind_session(
        &mut self,
        session: DesktopSessionMaterial,
    ) -> Result<(), DesktopShellError> {
        if !session.bound {
            return Err(DesktopShellError::SessionNotBound);
        }
        self.core
            .endpoint()
            .ok_or(DesktopShellError::SessionNotBound)?;
        let process_snapshot = self
            .core
            .bind_session(session.bound)
            .map_err(Self::map_process_adapter_error)?;
        self.observe_process_snapshot(&process_snapshot)?;
        self.core.apply_session_snapshot(&process_snapshot)?;
        self.state = DesktopServiceState::SessionBound;
        Ok(())
    }
}
