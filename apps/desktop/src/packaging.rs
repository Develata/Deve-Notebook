//! plan_ref:
//!   - 17_tech_stack#native-packaging-dependency-gate
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
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
pub const DESKTOP_MENU_APP_ID: &str = "deve-menu-app";
pub const DESKTOP_MENU_WINDOW_ID: &str = "deve-menu-window";
pub const DESKTOP_MENU_HELP_ID: &str = "deve-menu-help";
pub const DESKTOP_TRAY_ID: &str = "deve-tray";

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
    ProjectionWorkspace,
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
    pub menu_tray: DesktopMenuTraySurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopShellPackagingAcceptance {
    pub tauri_config_path: &'static str,
    pub product_name: &'static str,
    pub identifier: &'static str,
    pub main_window_label: &'static str,
    pub main_window_title: &'static str,
    pub runtime_entrypoint_declared: bool,
    pub build_script_declared: bool,
    pub menu_bar_runtime_declared: bool,
    pub system_tray_runtime_declared: bool,
    pub menu_and_tray_runtime_deferred: bool,
    pub installer_metadata_declared: bool,
    pub auto_update_artifacts_enabled: bool,
    pub session_handoff_required_before_writable_ui: bool,
    pub child_process_runtime_enabled: bool,
    pub release_ready_claimed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopMenuAction {
    ShowMainWindow,
    OpenCommandPalette,
    OpenSettings,
    UseLocalBackend,
    QuitRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopTrayAction {
    ShowMainWindow,
    ToggleWindowVisibility,
    UseLocalBackend,
    QuitRequested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopMenuTraySurface {
    pub app_menu_id: &'static str,
    pub window_menu_id: &'static str,
    pub help_menu_id: &'static str,
    pub tray_id: &'static str,
    pub menu_actions: &'static [DesktopMenuAction],
    pub tray_actions: &'static [DesktopTrayAction],
    pub menu_runtime_imported: bool,
    pub tray_runtime_imported: bool,
    pub actions_are_ui_intents_only: bool,
    pub opens_process_runtime: bool,
    pub process_runtime_changes_are_mode_coordinated: bool,
    pub opens_authority_write_path: bool,
}

impl DesktopMenuTraySurface {
    pub fn is_runtime_bound_authority_free(self) -> bool {
        self.menu_runtime_imported
            && self.tray_runtime_imported
            && self.actions_are_ui_intents_only
            && (!self.opens_process_runtime || self.process_runtime_changes_are_mode_coordinated)
            && !self.opens_authority_write_path
    }
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
        self.acceptance.shell.menu_bar_runtime_declared
            && self.acceptance.shell.runtime_entrypoint_declared
            && self.acceptance.shell.build_script_declared
            && self.acceptance.shell.system_tray_runtime_declared
            && !self.acceptance.shell.menu_and_tray_runtime_deferred
            && self.acceptance.shell.installer_metadata_declared
            && !self.acceptance.shell.auto_update_artifacts_enabled
            && self
                .acceptance
                .shell
                .session_handoff_required_before_writable_ui
            && self.acceptance.shell.child_process_runtime_enabled
            && !self.acceptance.shell.release_ready_claimed
            && self.acceptance.menu_tray.is_runtime_bound_authority_free()
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
            menu_tray: MENU_TRAY_SURFACE,
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
    runtime_entrypoint_declared: true,
    build_script_declared: true,
    menu_bar_runtime_declared: true,
    system_tray_runtime_declared: true,
    menu_and_tray_runtime_deferred: false,
    installer_metadata_declared: true,
    auto_update_artifacts_enabled: false,
    session_handoff_required_before_writable_ui: true,
    child_process_runtime_enabled: true,
    release_ready_claimed: false,
};

const MENU_TRAY_SURFACE: DesktopMenuTraySurface = DesktopMenuTraySurface {
    app_menu_id: DESKTOP_MENU_APP_ID,
    window_menu_id: DESKTOP_MENU_WINDOW_ID,
    help_menu_id: DESKTOP_MENU_HELP_ID,
    tray_id: DESKTOP_TRAY_ID,
    menu_actions: MENU_ACTIONS,
    tray_actions: TRAY_ACTIONS,
    menu_runtime_imported: true,
    tray_runtime_imported: true,
    actions_are_ui_intents_only: true,
    opens_process_runtime: true,
    process_runtime_changes_are_mode_coordinated: true,
    opens_authority_write_path: false,
};

const MENU_ACTIONS: &[DesktopMenuAction] = &[
    DesktopMenuAction::ShowMainWindow,
    DesktopMenuAction::OpenCommandPalette,
    DesktopMenuAction::OpenSettings,
    DesktopMenuAction::UseLocalBackend,
    DesktopMenuAction::QuitRequested,
];

const TRAY_ACTIONS: &[DesktopTrayAction] = &[
    DesktopTrayAction::ShowMainWindow,
    DesktopTrayAction::ToggleWindowVisibility,
    DesktopTrayAction::UseLocalBackend,
    DesktopTrayAction::QuitRequested,
];

const PACKAGING_CAPABILITIES: &[DesktopPackagingCapability] = &[
    DesktopPackagingCapability::WindowShell,
    DesktopPackagingCapability::MenuBar,
    DesktopPackagingCapability::SystemTray,
    DesktopPackagingCapability::Installer,
    DesktopPackagingCapability::AutoUpdate,
];

const FORBIDDEN_AUTHORITIES: &[DesktopPackagingAuthority] = &[
    DesktopPackagingAuthority::Ledger,
    DesktopPackagingAuthority::ProjectionWorkspace,
    DesktopPackagingAuthority::SourceControl,
    DesktopPackagingAuthority::SearchIndex,
    DesktopPackagingAuthority::GitMirror,
    DesktopPackagingAuthority::NoteGit,
];
