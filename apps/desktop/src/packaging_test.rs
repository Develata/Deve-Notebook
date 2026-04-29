//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate

use crate::{DesktopPackagingAuthority, DesktopPackagingCapability, desktop_packaging_scaffold};
use deve_core::native_adapter::CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

#[test]
fn desktop_packaging_scaffold_is_feature_gated_and_planned() {
    let scaffold = desktop_packaging_scaffold();
    let gate = CURRENT_NATIVE_PACKAGING_DEPENDENCY_GATE_POLICY;

    assert_eq!(scaffold.dependency_batch.feature_gate, "native-packaging");
    assert_eq!(scaffold.dependency_batch.runtime_crate, "tauri");
    assert_eq!(scaffold.dependency_batch.build_crate, "tauri-build");
    assert_eq!(scaffold.dependency_batch.status, "planned");
    assert!(scaffold.dependency_feature_is_isolated());
    assert!(gate.is_deferred_no_dependency());
    assert!(!gate.real_tauri_dependencies_allowed);
}

#[test]
fn desktop_packaging_acceptance_is_shell_only() {
    let scaffold = desktop_packaging_scaffold();

    assert_eq!(
        scaffold.acceptance.capabilities,
        [
            DesktopPackagingCapability::WindowShell,
            DesktopPackagingCapability::MenuBar,
            DesktopPackagingCapability::SystemTray,
            DesktopPackagingCapability::Installer,
            DesktopPackagingCapability::AutoUpdate,
        ]
    );
    assert_eq!(
        scaffold.acceptance.forbidden_authorities,
        [
            DesktopPackagingAuthority::Ledger,
            DesktopPackagingAuthority::Vault,
            DesktopPackagingAuthority::SourceControl,
            DesktopPackagingAuthority::SearchIndex,
            DesktopPackagingAuthority::GitMirror,
            DesktopPackagingAuthority::NoteGit,
        ]
    );
    assert!(scaffold.is_authority_free());
    assert!(scaffold.no_packaging_tests_remain_authoritative);
}
