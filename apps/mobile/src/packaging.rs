//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract
//!
//! Feature-gated mobile packaging scaffold.
//!
//! This module records the mobile packaging dependency batch without importing
//! or starting a native runtime process.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobilePackagingCapability {
    WebViewShell,
    PermissionBridge,
    ShareSheet,
    DeepLink,
    FilePicker,
    PushNotification,
    StorePackage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobilePackagingAuthority {
    Ledger,
    Vault,
    SourceControl,
    SearchIndex,
    GitMirror,
    NoteGit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobilePackagingDependencyBatch {
    pub feature_gate: &'static str,
    pub runtime_crate: &'static str,
    pub build_crate: &'static str,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobilePackagingAcceptance {
    pub capabilities: &'static [MobilePackagingCapability],
    pub forbidden_authorities: &'static [MobilePackagingAuthority],
    pub lifecycle_reprobe_remains_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobilePackagingScaffold {
    pub dependency_batch: MobilePackagingDependencyBatch,
    pub acceptance: MobilePackagingAcceptance,
    pub no_packaging_tests_remain_authoritative: bool,
}

impl MobilePackagingScaffold {
    pub fn is_authority_free(&self) -> bool {
        self.acceptance.forbidden_authorities == FORBIDDEN_AUTHORITIES
    }

    pub fn dependency_feature_is_isolated(&self) -> bool {
        self.dependency_batch.feature_gate == "native-packaging"
            && self.dependency_batch.status == "dependency-spike-open"
    }
}

pub fn mobile_packaging_scaffold() -> MobilePackagingScaffold {
    MobilePackagingScaffold {
        dependency_batch: MobilePackagingDependencyBatch {
            feature_gate: "native-packaging",
            runtime_crate: "tauri",
            build_crate: "tauri-build",
            status: "dependency-spike-open",
        },
        acceptance: MobilePackagingAcceptance {
            capabilities: PACKAGING_CAPABILITIES,
            forbidden_authorities: FORBIDDEN_AUTHORITIES,
            lifecycle_reprobe_remains_required: true,
        },
        no_packaging_tests_remain_authoritative: true,
    }
}

const PACKAGING_CAPABILITIES: &[MobilePackagingCapability] = &[
    MobilePackagingCapability::WebViewShell,
    MobilePackagingCapability::PermissionBridge,
    MobilePackagingCapability::ShareSheet,
    MobilePackagingCapability::DeepLink,
    MobilePackagingCapability::FilePicker,
    MobilePackagingCapability::PushNotification,
    MobilePackagingCapability::StorePackage,
];

const FORBIDDEN_AUTHORITIES: &[MobilePackagingAuthority] = &[
    MobilePackagingAuthority::Ledger,
    MobilePackagingAuthority::Vault,
    MobilePackagingAuthority::SourceControl,
    MobilePackagingAuthority::SearchIndex,
    MobilePackagingAuthority::GitMirror,
    MobilePackagingAuthority::NoteGit,
];
