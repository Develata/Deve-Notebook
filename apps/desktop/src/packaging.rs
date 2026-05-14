//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!
//! Feature-gated desktop packaging scaffold.
//!
//! This module describes the first packaging batch without importing a
//! packaging runtime. The actual runtime dependency remains a separate change.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopPackagingCapability {
    WindowShell,
    MenuBar,
    SystemTray,
    Installer,
    AutoUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopPackagingAuthority {
    Ledger,
    Vault,
    SourceControl,
    SearchIndex,
    GitMirror,
    NoteGit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopPackagingDependencyBatch {
    pub feature_gate: &'static str,
    pub runtime_crate: &'static str,
    pub build_crate: &'static str,
    pub status: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopPackagingAcceptance {
    pub capabilities: &'static [DesktopPackagingCapability],
    pub forbidden_authorities: &'static [DesktopPackagingAuthority],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopPackagingScaffold {
    pub dependency_batch: DesktopPackagingDependencyBatch,
    pub acceptance: DesktopPackagingAcceptance,
    pub no_packaging_tests_remain_authoritative: bool,
}

impl DesktopPackagingScaffold {
    pub fn is_authority_free(&self) -> bool {
        self.acceptance.forbidden_authorities == FORBIDDEN_AUTHORITIES
    }

    pub fn dependency_feature_is_isolated(&self) -> bool {
        self.dependency_batch.feature_gate == "native-packaging"
            && self.dependency_batch.status == "dependency-spike-open"
    }
}

pub fn desktop_packaging_scaffold() -> DesktopPackagingScaffold {
    DesktopPackagingScaffold {
        dependency_batch: DesktopPackagingDependencyBatch {
            feature_gate: "native-packaging",
            runtime_crate: "tauri",
            build_crate: "tauri-build",
            status: "dependency-spike-open",
        },
        acceptance: DesktopPackagingAcceptance {
            capabilities: PACKAGING_CAPABILITIES,
            forbidden_authorities: FORBIDDEN_AUTHORITIES,
        },
        no_packaging_tests_remain_authoritative: true,
    }
}

const PACKAGING_CAPABILITIES: &[DesktopPackagingCapability] = &[
    DesktopPackagingCapability::WindowShell,
    DesktopPackagingCapability::MenuBar,
    DesktopPackagingCapability::SystemTray,
    DesktopPackagingCapability::Installer,
    DesktopPackagingCapability::AutoUpdate,
];

const FORBIDDEN_AUTHORITIES: &[DesktopPackagingAuthority] = &[
    DesktopPackagingAuthority::Ledger,
    DesktopPackagingAuthority::Vault,
    DesktopPackagingAuthority::SourceControl,
    DesktopPackagingAuthority::SearchIndex,
    DesktopPackagingAuthority::GitMirror,
    DesktopPackagingAuthority::NoteGit,
];
