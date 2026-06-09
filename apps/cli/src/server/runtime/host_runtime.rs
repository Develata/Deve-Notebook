//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 05_diff_logic#source-control-runtime
//!
//! Plugin host API injection for server startup.

use deve_core::ledger::RepoManager;
use deve_core::plugin::runtime::host;
use std::path::PathBuf;
use std::sync::Arc;

use crate::server::{notegit, setup};

pub(crate) fn install_repo_host_apis(
    repo: &Arc<RepoManager>,
    git_bridge: deve_core::config::GitBridgeMode,
) -> anyhow::Result<()> {
    let repo_api: Arc<dyn deve_core::ledger::traits::Repository> = repo.clone();
    host::set_repository(repo_api)?;
    let source_control_api: Arc<dyn deve_core::source_control::SourceControlApi> = repo.clone();
    host::set_source_control_api(source_control_api, git_bridge)?;
    host::set_repo_manager(repo.clone())?;
    Ok(())
}

pub(crate) fn install_sync_host_api(
    sync_manager: Arc<deve_core::sync::SyncManager>,
) -> anyhow::Result<()> {
    host::set_sync_manager(sync_manager)?;
    Ok(())
}

pub(crate) fn prepare_host_layout(repo: &RepoManager, port: u16) -> anyhow::Result<PathBuf> {
    let host_dir = notegit::prepare(repo)?;
    setup::write_main_port_hint(&host_dir, port)?;
    Ok(host_dir)
}
