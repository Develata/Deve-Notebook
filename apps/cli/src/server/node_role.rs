// apps/cli/src/server/node_role.rs
//! plan_ref:
//!   - 07_network#full-peer-mesh-v1
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 18_release#runtime-observability
//!
//! # Node Role State

use deve_core::config::RuntimeEnvironment;
use deve_core::native_adapter::{NativeEndpointReady, NativeServiceOffline};
use std::sync::{Arc, OnceLock, RwLock};

#[derive(Clone, Debug)]
pub struct NodeRole {
    pub role: String,
    pub ws_port: u16,
    pub main_port: u16,
    pub version: String,
    pub profile: String,
    pub delivery: String,
    pub environment: String,
    pub repo_health: RepoHealthSummary,
    pub source_control: SourceControlSummary,
    pub p2p: P2pSummary,
    pub native_service: Option<NativeServiceSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepoHealthSummary {
    pub status: String,
    pub local_total: usize,
    pub healthy: usize,
    pub degraded: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WatcherHealthSummary {
    pub status: String,
    pub expected: usize,
    pub running: usize,
    pub unavailable: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeServiceSummary {
    pub state: String,
    pub endpoint: Option<NativeEndpointReady>,
    pub offline: Option<NativeServiceOffline>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceControlSummary {
    pub authority: String,
    pub git_main_mirror: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFileActionSummary {
    pub copy_absolute_path: bool,
    pub reveal_in_system_explorer: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct P2pSummary {
    pub enabled: bool,
    pub peers: Vec<P2pPeerSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct P2pPeerSummary {
    pub label: String,
    pub peer_id: String,
    pub repo_id: String,
    pub state: String,
    pub attempts: u64,
    pub handshakes: u64,
    pub sent_pushes: u64,
    pub sent_snapshots: u64,
    pub applied_pushes: u64,
    pub applied_snapshots: u64,
    pub last_error_code: Option<String>,
}

impl RepoHealthSummary {
    pub fn unknown() -> Self {
        Self {
            status: "unknown".into(),
            local_total: 0,
            healthy: 0,
            degraded: 0,
        }
    }

    pub fn from_degraded_count(local_total: usize, degraded: usize) -> Self {
        let degraded = degraded.min(local_total);
        let healthy = local_total - degraded;
        let status = if degraded > 0 { "degraded" } else { "healthy" };
        Self {
            status: status.into(),
            local_total,
            healthy,
            degraded,
        }
    }
}

impl WatcherHealthSummary {
    pub fn unknown() -> Self {
        Self {
            status: "unknown".into(),
            expected: 0,
            running: 0,
            unavailable: 0,
        }
    }

    pub(crate) fn from_aggregate(
        aggregate: crate::server::runtime::watcher_runtime::WatcherRuntimeAggregate,
    ) -> Self {
        Self {
            status: aggregate.status.as_str().into(),
            expected: aggregate.expected,
            running: aggregate.running,
            unavailable: aggregate.unavailable,
        }
    }
}

impl SourceControlSummary {
    pub fn ngit_authority() -> Self {
        Self {
            authority: "ngit".into(),
            git_main_mirror: "main".into(),
        }
    }

    pub fn unknown() -> Self {
        Self {
            authority: "unknown".into(),
            git_main_mirror: "unknown".into(),
        }
    }
}

impl P2pSummary {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            peers: Vec::new(),
        }
    }
}

pub fn host_file_actions_for(role: &NodeRole) -> HostFileActionSummary {
    let local_host =
        role.role == "native-main" || (role.role == "main" && role.environment == "development");
    HostFileActionSummary {
        copy_absolute_path: local_host,
        reveal_in_system_explorer: local_host && system_file_manager_reveal_available(),
    }
}

fn system_file_manager_reveal_available() -> bool {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        true
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some()
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        false
    }
}

static NODE_ROLE: OnceLock<Arc<RwLock<NodeRole>>> = OnceLock::new();

pub fn set_node_role(role: NodeRole) {
    let cell = role_cell();
    match cell.write() {
        Ok(mut current) => *current = role,
        Err(_) => tracing::warn!("NodeRole lock poisoned, ignoring role update"),
    }
}

pub fn update_repo_health(repo_health: RepoHealthSummary) {
    let cell = role_cell();
    match cell.write() {
        Ok(mut current) => current.repo_health = repo_health,
        Err(_) => tracing::warn!("NodeRole lock poisoned, ignoring repo health update"),
    }
}

pub fn update_p2p_summary(p2p: P2pSummary) {
    let cell = role_cell();
    match cell.write() {
        Ok(mut current) => current.p2p = p2p,
        Err(_) => tracing::warn!("NodeRole lock poisoned, ignoring P2P summary update"),
    }
}

pub fn mark_native_session_bound() {
    let cell = role_cell();
    match cell.write() {
        Ok(mut current) => {
            if let Some(service) = current.native_service.as_mut() {
                service.state = "endpoint_ready".into();
                if let Some(endpoint) = service.endpoint.as_mut() {
                    endpoint.session_bound = true;
                }
            }
        }
        Err(_) => tracing::warn!("NodeRole lock poisoned, ignoring native session update"),
    }
}

pub fn get_node_role() -> NodeRole {
    let cell = role_cell();
    match cell.read() {
        Ok(current) => current.clone(),
        Err(_) => {
            tracing::warn!("NodeRole lock poisoned, returning unknown role");
            default_node_role()
        }
    }
}

pub fn runtime_environment() -> String {
    RuntimeEnvironment::from_env().as_str().into()
}

#[cfg(test)]
fn runtime_environment_label(env: Option<&str>) -> &'static str {
    RuntimeEnvironment::from_deve_env(env).as_str()
}

fn role_cell() -> Arc<RwLock<NodeRole>> {
    NODE_ROLE
        .get_or_init(|| Arc::new(RwLock::new(default_node_role())))
        .clone()
}

fn default_node_role() -> NodeRole {
    NodeRole {
        role: "unknown".into(),
        ws_port: 0,
        main_port: 0,
        version: env!("CARGO_PKG_VERSION").into(),
        profile: "unknown".into(),
        delivery: "unknown".into(),
        environment: runtime_environment(),
        repo_health: RepoHealthSummary::unknown(),
        source_control: SourceControlSummary::unknown(),
        p2p: P2pSummary::disabled(),
        native_service: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NodeRole, P2pSummary, RepoHealthSummary, SourceControlSummary, WatcherHealthSummary,
        host_file_actions_for, runtime_environment_label,
    };

    #[test]
    fn unknown_repo_health_uses_zero_counts() {
        assert_eq!(
            RepoHealthSummary::unknown(),
            RepoHealthSummary {
                status: "unknown".into(),
                local_total: 0,
                healthy: 0,
                degraded: 0,
            }
        );
    }

    #[test]
    fn unknown_watcher_health_uses_zero_counts() {
        assert_eq!(
            WatcherHealthSummary::unknown(),
            WatcherHealthSummary {
                status: "unknown".into(),
                expected: 0,
                running: 0,
                unavailable: 0,
            }
        );
    }

    #[test]
    fn from_degraded_count_reports_healthy_without_degraded_repos() {
        assert_eq!(
            RepoHealthSummary::from_degraded_count(2, 0),
            RepoHealthSummary {
                status: "healthy".into(),
                local_total: 2,
                healthy: 2,
                degraded: 0,
            }
        );
    }

    #[test]
    fn from_degraded_count_reports_degraded_repos() {
        assert_eq!(
            RepoHealthSummary::from_degraded_count(2, 1),
            RepoHealthSummary {
                status: "degraded".into(),
                local_total: 2,
                healthy: 1,
                degraded: 1,
            }
        );
    }

    #[test]
    fn from_degraded_count_clamps_degraded_count_to_local_total() {
        assert_eq!(
            RepoHealthSummary::from_degraded_count(2, 3),
            RepoHealthSummary {
                status: "degraded".into(),
                local_total: 2,
                healthy: 0,
                degraded: 2,
            }
        );
    }

    #[test]
    fn source_control_summary_reports_ngit_authority() {
        let summary = SourceControlSummary::ngit_authority();
        assert_eq!(summary.authority, "ngit");
        assert_eq!(summary.git_main_mirror, "main");
        assert_eq!(SourceControlSummary::unknown().authority, "unknown");
    }

    #[test]
    fn host_file_actions_are_local_dev_or_native_only() {
        let mut role = NodeRole {
            role: "main".into(),
            ws_port: 3001,
            main_port: 3001,
            version: "0.0.1".into(),
            profile: "standard".into(),
            delivery: "embedded-frontend".into(),
            environment: "production".into(),
            repo_health: RepoHealthSummary::unknown(),
            source_control: SourceControlSummary::unknown(),
            p2p: P2pSummary::disabled(),
            native_service: None,
        };
        assert!(!host_file_actions_for(&role).copy_absolute_path);

        role.environment = "development".into();
        assert!(host_file_actions_for(&role).copy_absolute_path);

        role.role = "proxy".into();
        assert!(!host_file_actions_for(&role).copy_absolute_path);

        role.role = "native-main".into();
        role.environment = "production".into();
        assert!(host_file_actions_for(&role).copy_absolute_path);
    }

    #[test]
    fn runtime_environment_label_reports_effective_mode() {
        assert_eq!(
            runtime_environment_label(Some("development")),
            "development"
        );
        assert_eq!(
            runtime_environment_label(Some(" Development ")),
            "development"
        );
        assert_eq!(runtime_environment_label(Some("production")), "production");
        assert_eq!(runtime_environment_label(Some("staging")), "production");
        assert_eq!(runtime_environment_label(None), "production");
    }
}
