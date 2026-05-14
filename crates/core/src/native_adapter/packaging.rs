//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate
//!   - 08_ui_design_02_desktop#desktop-packaging-dependency-gate-decision
//!   - 08_ui_design_03_mobile#mobile-packaging-dependency-gate-decision

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePackagingDependencyGateDecision {
    DeferredUntilRuntimeBatch,
    DesktopDependencySpikeOpen,
    DesktopAndMobileDependencySpikeOpen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativePackagingDependencyGatePolicy {
    pub decision: NativePackagingDependencyGateDecision,
    pub desktop_tauri_dependencies_allowed: bool,
    pub mobile_tauri_dependencies_allowed: bool,
    pub default_build_remains_no_tauri: bool,
    pub native_feature_gate_required: bool,
    pub authority_writes_allowed: bool,
}

impl NativePackagingDependencyGatePolicy {
    pub fn is_desktop_dependency_spike_open(self) -> bool {
        matches!(
            self.decision,
            NativePackagingDependencyGateDecision::DesktopDependencySpikeOpen
                | NativePackagingDependencyGateDecision::DesktopAndMobileDependencySpikeOpen
        ) && self.desktop_tauri_dependencies_allowed
            && self.default_build_remains_no_tauri
            && self.native_feature_gate_required
            && !self.authority_writes_allowed
    }

    pub fn is_mobile_dependency_spike_open(self) -> bool {
        self.decision == NativePackagingDependencyGateDecision::DesktopAndMobileDependencySpikeOpen
            && self.desktop_tauri_dependencies_allowed
            && self.mobile_tauri_dependencies_allowed
            && self.default_build_remains_no_tauri
            && self.native_feature_gate_required
            && !self.authority_writes_allowed
    }

    pub fn mobile_packaging_stays_deferred(self) -> bool {
        !self.mobile_tauri_dependencies_allowed
            && self.default_build_remains_no_tauri
            && self.native_feature_gate_required
            && !self.authority_writes_allowed
    }
}

pub const CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY: NativePackagingDependencyGatePolicy =
    NativePackagingDependencyGatePolicy {
        decision: NativePackagingDependencyGateDecision::DesktopAndMobileDependencySpikeOpen,
        desktop_tauri_dependencies_allowed: true,
        mobile_tauri_dependencies_allowed: true,
        default_build_remains_no_tauri: true,
        native_feature_gate_required: true,
        authority_writes_allowed: false,
    };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_policy_opens_desktop_and_mobile_dependency_spikes() {
        let policy = CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

        assert!(policy.is_desktop_dependency_spike_open());
        assert!(policy.is_mobile_dependency_spike_open());
        assert!(policy.desktop_tauri_dependencies_allowed);
        assert!(policy.mobile_tauri_dependencies_allowed);
        assert!(!policy.mobile_packaging_stays_deferred());
        assert!(policy.default_build_remains_no_tauri);
        assert!(policy.native_feature_gate_required);
        assert!(!policy.authority_writes_allowed);
    }
}
