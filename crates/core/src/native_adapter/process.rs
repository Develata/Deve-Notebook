//! plan_ref:
//!   - 08_ui_design_02_desktop#desktop-process-adapter-decision
//!   - 08_ui_design_03_mobile#mobile-process-adapter-decision

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NativeProcessAdapterDecision {
    DeferredUntilPackagingGate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeProcessAdapterPolicy {
    pub decision: NativeProcessAdapterDecision,
    pub child_process_runtime_enabled: bool,
    pub packaging_gate_required: bool,
    pub authority_writes_allowed: bool,
}

impl NativeProcessAdapterPolicy {
    pub fn is_deferred_no_runtime(self) -> bool {
        self.decision == NativeProcessAdapterDecision::DeferredUntilPackagingGate
            && !self.child_process_runtime_enabled
            && self.packaging_gate_required
            && !self.authority_writes_allowed
    }
}

pub const CURRENT_NATIVE_PROCESS_ADAPTER_POLICY: NativeProcessAdapterPolicy =
    NativeProcessAdapterPolicy {
        decision: NativeProcessAdapterDecision::DeferredUntilPackagingGate,
        child_process_runtime_enabled: false,
        packaging_gate_required: true,
        authority_writes_allowed: false,
    };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_policy_defers_real_process_runtime() {
        let policy = CURRENT_NATIVE_PROCESS_ADAPTER_POLICY;

        assert!(policy.is_deferred_no_runtime());
        assert!(!policy.child_process_runtime_enabled);
        assert!(policy.packaging_gate_required);
        assert!(!policy.authority_writes_allowed);
    }
}
