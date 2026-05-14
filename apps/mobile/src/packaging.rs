//! plan_ref:
//!   - 14_tech_stack#native-packaging-dependency-gate
//!   - 08_ui_design_03_mobile#mobile-native-adapter-contract
//!   - 08_ui_design_03_mobile#mobile-android-shell-package-execution-gate
//!
//! Feature-gated mobile packaging scaffold.
//!
//! This module records the mobile packaging dependency batch and Android
//! shell-only package gate without starting a backend process.

pub const MOBILE_TAURI_CONFIG_PATH: &str = "apps/mobile/tauri.conf.json";
pub const MOBILE_TAURI_PRODUCT_NAME: &str = "Deve Notebook";
pub const MOBILE_TAURI_IDENTIFIER: &str = "dev.deve.notebook.mobile";
pub const MOBILE_TAURI_MAIN_WINDOW_LABEL: &str = "main";
pub const MOBILE_TAURI_MAIN_WINDOW_TITLE: &str = "Deve Notebook";
pub const MOBILE_ANDROID_PACKAGE_SCRIPT: &str =
    "scripts/check-mobile-android-shell-package-build.sh";
pub const MOBILE_ANDROID_PACKAGE_GATE_ANCHOR: &str =
    "08_ui_design_03_mobile#mobile-android-shell-package-execution-gate";

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
    pub shell: MobileShellPackagingAcceptance,
    pub android_shell_package: MobileAndroidShellPackageExecution,
    pub lifecycle_reprobe_remains_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobileShellPackagingAcceptance {
    pub tauri_config_path: &'static str,
    pub product_name: &'static str,
    pub identifier: &'static str,
    pub main_window_label: &'static str,
    pub main_window_title: &'static str,
    pub manifest_declared: bool,
    pub build_script_declared: bool,
    pub android_project_generated: bool,
    pub ios_project_generated: bool,
    pub runtime_entrypoint_declared: bool,
    pub platform_package_build_declared: bool,
    pub session_handoff_required_before_writable_ui: bool,
    pub foreground_reprobe_required: bool,
    pub child_process_runtime_enabled: bool,
    pub release_ready_claimed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MobileAndroidShellPackageExecution {
    pub gate_anchor: &'static str,
    pub target_host_script: &'static str,
    pub project_generation_allowed: bool,
    pub package_build_allowed: bool,
    pub ios_package_build_allowed: bool,
    pub child_process_runtime_enabled: bool,
    pub opens_authority_write_path: bool,
    pub release_ready_claimed: bool,
}

impl MobileAndroidShellPackageExecution {
    pub fn is_shell_only_open(self) -> bool {
        self.project_generation_allowed
            && self.package_build_allowed
            && !self.ios_package_build_allowed
            && !self.child_process_runtime_enabled
            && !self.opens_authority_write_path
            && !self.release_ready_claimed
    }
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

    pub fn shell_acceptance_is_authority_free(&self) -> bool {
        self.acceptance.shell.manifest_declared
            && self.acceptance.shell.build_script_declared
            && !self.acceptance.shell.ios_project_generated
            && self.acceptance.shell.runtime_entrypoint_declared
            && self.acceptance.shell.platform_package_build_declared
            && self
                .acceptance
                .shell
                .session_handoff_required_before_writable_ui
            && self.acceptance.shell.foreground_reprobe_required
            && !self.acceptance.shell.child_process_runtime_enabled
            && !self.acceptance.shell.release_ready_claimed
            && self.acceptance.android_shell_package.is_shell_only_open()
            && self.is_authority_free()
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
            shell: SHELL_ACCEPTANCE,
            android_shell_package: ANDROID_SHELL_PACKAGE_EXECUTION,
            lifecycle_reprobe_remains_required: true,
        },
        no_packaging_tests_remain_authoritative: true,
    }
}

const SHELL_ACCEPTANCE: MobileShellPackagingAcceptance = MobileShellPackagingAcceptance {
    tauri_config_path: MOBILE_TAURI_CONFIG_PATH,
    product_name: MOBILE_TAURI_PRODUCT_NAME,
    identifier: MOBILE_TAURI_IDENTIFIER,
    main_window_label: MOBILE_TAURI_MAIN_WINDOW_LABEL,
    main_window_title: MOBILE_TAURI_MAIN_WINDOW_TITLE,
    manifest_declared: true,
    build_script_declared: true,
    android_project_generated: true,
    ios_project_generated: false,
    runtime_entrypoint_declared: true,
    platform_package_build_declared: true,
    session_handoff_required_before_writable_ui: true,
    foreground_reprobe_required: true,
    child_process_runtime_enabled: false,
    release_ready_claimed: false,
};

const ANDROID_SHELL_PACKAGE_EXECUTION: MobileAndroidShellPackageExecution =
    MobileAndroidShellPackageExecution {
        gate_anchor: MOBILE_ANDROID_PACKAGE_GATE_ANCHOR,
        target_host_script: MOBILE_ANDROID_PACKAGE_SCRIPT,
        project_generation_allowed: true,
        package_build_allowed: true,
        ios_package_build_allowed: false,
        child_process_runtime_enabled: false,
        opens_authority_write_path: false,
        release_ready_claimed: false,
    };

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
