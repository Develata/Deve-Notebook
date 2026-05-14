//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate

use crate::{MobilePackagingAuthority, MobilePackagingCapability, mobile_packaging_scaffold};
use deve_core::native_adapter::CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

#[test]
fn mobile_packaging_dependency_spike_is_feature_gated() {
    let scaffold = mobile_packaging_scaffold();
    let gate = CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

    assert_eq!(scaffold.dependency_batch.feature_gate, "native-packaging");
    assert_eq!(scaffold.dependency_batch.runtime_crate, "tauri");
    assert_eq!(scaffold.dependency_batch.build_crate, "tauri-build");
    assert_eq!(scaffold.dependency_batch.status, "dependency-spike-open");
    assert!(scaffold.dependency_feature_is_isolated());
    assert!(gate.is_mobile_dependency_spike_open());
    assert!(gate.mobile_tauri_dependencies_allowed);
    assert!(!gate.mobile_packaging_stays_deferred());
}

#[test]
fn mobile_packaging_acceptance_is_shell_only() {
    let scaffold = mobile_packaging_scaffold();

    assert_eq!(
        scaffold.acceptance.capabilities,
        [
            MobilePackagingCapability::WebViewShell,
            MobilePackagingCapability::PermissionBridge,
            MobilePackagingCapability::ShareSheet,
            MobilePackagingCapability::DeepLink,
            MobilePackagingCapability::FilePicker,
            MobilePackagingCapability::PushNotification,
            MobilePackagingCapability::StorePackage,
        ]
    );
    assert_eq!(
        scaffold.acceptance.forbidden_authorities,
        [
            MobilePackagingAuthority::Ledger,
            MobilePackagingAuthority::Vault,
            MobilePackagingAuthority::SourceControl,
            MobilePackagingAuthority::SearchIndex,
            MobilePackagingAuthority::GitMirror,
            MobilePackagingAuthority::NoteGit,
        ]
    );
    assert!(scaffold.is_authority_free());
    assert!(scaffold.acceptance.lifecycle_reprobe_remains_required);
    assert!(scaffold.no_packaging_tests_remain_authoritative);
}
