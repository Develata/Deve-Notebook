//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 18_release#runtime-observability
//!
//! Node role and repo-health runtime assembly.

use crate::server::{launch::ServerLaunchOptions, node_role, static_files};
use deve_core::config::AppProfile;
use deve_core::ledger::RepoManager;

pub(crate) fn init_node_role(launch: &ServerLaunchOptions, profile: AppProfile) {
    let port = launch.port();
    node_role::set_node_role(node_role::NodeRole {
        role: launch.node_role_label().into(),
        ws_port: port,
        main_port: port,
        version: env!("CARGO_PKG_VERSION").into(),
        profile: profile_label(profile).into(),
        delivery: static_files::delivery_shape().into(),
        environment: launch.runtime_environment().as_str().into(),
        repo_health: node_role::RepoHealthSummary::unknown(),
        source_control: node_role::SourceControlSummary::ngit_authority(),
        p2p: node_role::P2pSummary::disabled(),
        native_service: launch.native_service_summary(),
    });
}

pub(crate) fn update_repo_health(repo: &RepoManager, sync_manager: &deve_core::sync::SyncManager) {
    node_role::update_repo_health(current_repo_health(repo, sync_manager));
}

fn profile_label(profile: AppProfile) -> &'static str {
    match profile {
        AppProfile::Standard => "standard",
        AppProfile::LowSpec => "low-spec",
    }
}

pub(crate) fn current_repo_health(
    repo: &RepoManager,
    sync_manager: &deve_core::sync::SyncManager,
) -> node_role::RepoHealthSummary {
    let local_total = match repo.list_local_repo_names_for_execution() {
        Ok(repos) => repos.len(),
        Err(err) => {
            tracing::warn!("Failed to list repos for node role health: {}", err);
            return node_role::RepoHealthSummary::unknown();
        }
    };
    match sync_manager.degraded_local_repo_names_for_execution() {
        Ok(degraded) => {
            node_role::RepoHealthSummary::from_degraded_count(local_total, degraded.len())
        }
        Err(err) => {
            tracing::warn!("Failed to summarize repo health for node role: {}", err);
            node_role::RepoHealthSummary::unknown()
        }
    }
}
