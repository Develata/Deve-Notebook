//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate
//!   - 08_ui_design_02_desktop#desktop-native-adapter-contract
//!
//! Feature-gated desktop packaging scaffold.
//!
//! This module records the desktop shell packaging acceptance surface without
//! importing or starting a native runtime process.

pub const DESKTOP_TAURI_CONFIG_PATH: &str = "apps/desktop/tauri.conf.json";
pub const DESKTOP_TAURI_PRODUCT_NAME: &str = "Deve Notebook";
pub const DESKTOP_TAURI_IDENTIFIER: &str = "dev.deve.notebook";
pub const DESKTOP_TAURI_MAIN_WINDOW_LABEL: &str = "main";
pub const DESKTOP_TAURI_MAIN_WINDOW_TITLE: &str = "Deve Notebook";

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
    pub shell: DesktopShellPackagingAcceptance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopShellPackagingAcceptance {
    pub tauri_config_path: &'static str,
    pub product_name: &'static str,
    pub identifier: &'static str,
    pub main_window_label: &'static str,
    pub main_window_title: &'static str,
    pub menu_bar_runtime_declared: bool,
    pub system_tray_runtime_declared: bool,
    pub menu_and_tray_runtime_deferred: bool,
    pub installer_metadata_declared: bool,
    pub auto_update_artifacts_enabled: bool,
    pub session_handoff_required_before_writable_ui: bool,
    pub child_process_runtime_enabled: bool,
    pub release_ready_claimed: bool,
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

    pub fn shell_acceptance_is_authority_free(&self) -> bool {
        !self.acceptance.shell.menu_bar_runtime_declared
            && !self.acceptance.shell.system_tray_runtime_declared
            && self.acceptance.shell.menu_and_tray_runtime_deferred
            && self.acceptance.shell.installer_metadata_declared
            && !self.acceptance.shell.auto_update_artifacts_enabled
            && self
                .acceptance
                .shell
                .session_handoff_required_before_writable_ui
            && !self.acceptance.shell.child_process_runtime_enabled
            && !self.acceptance.shell.release_ready_claimed
            && self.is_authority_free()
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
            shell: SHELL_ACCEPTANCE,
        },
        no_packaging_tests_remain_authoritative: true,
    }
}

const SHELL_ACCEPTANCE: DesktopShellPackagingAcceptance = DesktopShellPackagingAcceptance {
    tauri_config_path: DESKTOP_TAURI_CONFIG_PATH,
    product_name: DESKTOP_TAURI_PRODUCT_NAME,
    identifier: DESKTOP_TAURI_IDENTIFIER,
    main_window_label: DESKTOP_TAURI_MAIN_WINDOW_LABEL,
    main_window_title: DESKTOP_TAURI_MAIN_WINDOW_TITLE,
    menu_bar_runtime_declared: false,
    system_tray_runtime_declared: false,
    menu_and_tray_runtime_deferred: true,
    installer_metadata_declared: true,
    auto_update_artifacts_enabled: false,
    session_handoff_required_before_writable_ui: true,
    child_process_runtime_enabled: false,
    release_ready_claimed: false,
};

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
