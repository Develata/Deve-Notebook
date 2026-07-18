//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 07_network#server-ws-runtime
//!   - 18_release#runtime-observability
//!
//! Node role and repo-health runtime assembly.

use crate::server::runtime::watcher_runtime::WatcherRuntimeView;
use crate::server::{launch::ServerLaunchOptions, node_role, static_files};
use deve_core::config::AppProfile;
use deve_core::ledger::RepoManager;
use std::collections::HashSet;

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

pub(crate) fn current_watcher_health(
    repo: &RepoManager,
    sync_manager: &deve_core::sync::SyncManager,
    watcher_runtime: &WatcherRuntimeView,
) -> node_role::WatcherHealthSummary {
    let summaries = match repo.list_local_repo_summaries() {
        Ok(summaries) => summaries,
        Err(error) => {
            tracing::warn!(%error, "Failed to list expected repos for watcher health");
            return node_role::WatcherHealthSummary::unknown();
        }
    };
    let degraded = match sync_manager.degraded_local_repo_names_for_execution() {
        Ok(degraded) => degraded.into_iter().collect::<HashSet<_>>(),
        Err(error) => {
            tracing::warn!(%error, "Failed to snapshot projection health for watcher health");
            return node_role::WatcherHealthSummary::unknown();
        }
    };
    let expected = summaries
        .into_iter()
        .filter(|summary| !degraded.contains(&summary.execution_name))
        .map(|summary| summary.repo_id)
        .collect::<HashSet<_>>();
    node_role::WatcherHealthSummary::from_aggregate(watcher_runtime.aggregate(&expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::runtime::watcher_runtime::RepoMountState;
    use std::sync::Arc;

    #[test]
    fn node_role_watcher_health_uses_only_healthy_local_expected_set() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let projection_base = dir.path().join("notes");
        std::fs::create_dir_all(&projection_base)?;
        let repo = RepoManager::init(dir.path().join("ledger"), 8, Some("main"), Some("urn:main"))?;
        repo.set_projection_base_for_local_repo("main", &projection_base)?;
        let repo = Arc::new(repo);
        let sync = Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?);
        let repo_id = repo
            .get_repo_info_for(None, Some("main"))?
            .expect("main repo")
            .uuid;
        let view = WatcherRuntimeView::with_state_for_test(repo_id, 1, RepoMountState::Mounted);

        let healthy = current_watcher_health(repo.as_ref(), sync.as_ref(), &view);
        assert_eq!(healthy.status, "healthy");
        assert_eq!(healthy.expected, 1);
        assert_eq!(healthy.running, 1);
        assert_eq!(healthy.unavailable, 0);

        sync.mark_projection_writeback_fault(repo.local_repo_name());
        view.set_state_for_test(repo_id, RepoMountState::Failed);
        let degraded_projection = current_watcher_health(repo.as_ref(), sync.as_ref(), &view);
        assert_eq!(degraded_projection.status, "healthy");
        assert_eq!(degraded_projection.expected, 0);
        assert_eq!(degraded_projection.running, 0);
        assert_eq!(degraded_projection.unavailable, 0);
        Ok(())
    }
}
