//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate
//!   - 08_ui_design_02_desktop#desktop-packaging-dependency-gate-decision
//!   - 08_ui_design_03_mobile#mobile-packaging-dependency-gate-decision

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativePackagingDependencyGateDecision {
    DeferredUntilRuntimeBatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativePackagingDependencyGatePolicy {
    pub decision: NativePackagingDependencyGateDecision,
    pub real_tauri_dependencies_allowed: bool,
    pub default_build_remains_no_tauri: bool,
    pub native_feature_gate_required: bool,
    pub authority_writes_allowed: bool,
}

impl NativePackagingDependencyGatePolicy {
    pub fn is_deferred_no_dependency(self) -> bool {
        self.decision == NativePackagingDependencyGateDecision::DeferredUntilRuntimeBatch
            && !self.real_tauri_dependencies_allowed
            && self.default_build_remains_no_tauri
            && self.native_feature_gate_required
            && !self.authority_writes_allowed
    }
}

pub const CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY: NativePackagingDependencyGatePolicy =
    NativePackagingDependencyGatePolicy {
        decision: NativePackagingDependencyGateDecision::DeferredUntilRuntimeBatch,
        real_tauri_dependencies_allowed: false,
        default_build_remains_no_tauri: true,
        native_feature_gate_required: true,
        authority_writes_allowed: false,
    };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_policy_defers_real_tauri_dependencies() {
        let policy = CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

        assert!(policy.is_deferred_no_dependency());
        assert!(!policy.real_tauri_dependencies_allowed);
        assert!(policy.default_build_remains_no_tauri);
        assert!(policy.native_feature_gate_required);
        assert!(!policy.authority_writes_allowed);
    }
}
