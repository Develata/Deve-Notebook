//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract

use deve_core::native_adapter::{
    NativePlatformEventKind, NativeRuntimeReadiness, NativeServiceSuspended,
};

use super::MobileShell;
use crate::types::{MobileLifecycleEvent, MobileServiceState};

impl MobileShell {
    pub fn mark_runtime_ready(&mut self, readiness: NativeRuntimeReadiness) -> bool {
        if self.core.terminal_offline_reason().is_some() {
            return false;
        }
        let ready = self.core.set_runtime_readiness(readiness);
        if ready {
            self.state = MobileServiceState::RuntimeReady;
            self.suspended = None;
        }
        ready
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

    pub fn invalidate_session(&mut self) {
        self.state = MobileServiceState::SessionInvalid;
        self.core.invalidate_session_binding();
    }

    fn require_foreground_reprobe(&mut self) {
        self.state = MobileServiceState::ForegroundReprobe;
        self.suspended = None;
        self.core.require_foreground_reprobe();
    }

    fn is_service_recovery_state(&self) -> bool {
        matches!(
            self.state,
            MobileServiceState::ServiceRestarting | MobileServiceState::ServiceOffline
        )
    }
}
