//! plan_ref:
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract

use deve_core::native_adapter::{
    NativeAdapterPlatform, NativePlatformEventEffect, NativePlatformEventKind,
    NativeRuntimeReadiness, classify_native_platform_event,
};

use super::DesktopShell;
use crate::types::DesktopServiceState;

impl DesktopShell {
    pub fn mark_runtime_ready(&mut self, readiness: NativeRuntimeReadiness) -> bool {
        if self.core.terminal_offline_reason().is_some() {
            return false;
        }
        let ready = self.core.set_runtime_readiness(readiness);
        if ready {
            self.state = DesktopServiceState::RuntimeReady;
        }
        ready
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

    pub fn invalidate_session(&mut self) {
        self.state = DesktopServiceState::SessionInvalid;
        self.core.invalidate_session_binding();
    }

    fn require_foreground_reprobe(&mut self) {
        self.state = DesktopServiceState::ForegroundReprobe;
        self.core.require_foreground_reprobe();
    }

    fn is_service_recovery_state(&self) -> bool {
        matches!(
            self.state,
            DesktopServiceState::ServiceRestarting | DesktopServiceState::ServiceOffline
        )
    }
}
