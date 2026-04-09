use crate::server::AppState;
use crate::server::session::WsSession;
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::RepoId;
use std::sync::Arc;

#[path = "switcher_prepare_branch.rs"]
mod branch;
#[path = "switcher_prepare_recovery.rs"]
mod recovery;

pub(super) struct PreparedRepoSwitch {
    pub repo_name: String,
    pub repo_id: Option<RepoId>,
    pub db: Option<DatabaseHandle>,
    pub degraded_docs_only: bool,
}

pub(crate) use branch::validate_branch_target;

pub(super) fn prepare_repo_switch(
    state: &Arc<AppState>,
    branch: Option<&deve_core::models::PeerId>,
    repo_name: String,
) -> anyhow::Result<PreparedRepoSwitch> {
    let repo_info = state
        .repo
        .get_repo_info_for(branch, Some(&repo_name))?
        .map(|info| info.uuid);
    if repo_info.is_none() {
        let scope = if branch.is_some() { "Remote" } else { "Local" };
        return Err(anyhow::anyhow!(
            "{scope} repository UUID not resolved for selector: {}",
            repo_name
        ));
    }
    if branch.is_some() {
        let handle = state.repo.open_database(branch, &repo_name)?;
        return Ok(PreparedRepoSwitch {
            repo_name,
            repo_id: repo_info,
            db: Some(handle),
            degraded_docs_only: false,
        });
    }
    let degraded_docs_only = match state.sync_manager.materialize_local_repo(&repo_name) {
        Ok(()) => false,
        Err(err) if recovery::should_degrade_local_projection(&err) => {
            tracing::warn!(
                repo_name = %repo_name,
                error = %err,
                "Local repo switch degraded to docs-only fallback due to broken structure projection"
            );
            true
        }
        Err(err) => return Err(err),
    };
    Ok(PreparedRepoSwitch {
        repo_name,
        repo_id: repo_info,
        db: None,
        degraded_docs_only,
    })
}

pub(super) fn commit_session_switch(
    session: &mut WsSession,
    branch: Option<String>,
    prepared: Option<PreparedRepoSwitch>,
    scope_nonce: Option<u64>,
) {
    session.set_scope_nonce(scope_nonce);
    session.clear_sync_binding();
    session.switch_branch(branch);
    match prepared {
        Some(prepared) => {
            session.switch_repo(prepared.repo_name, prepared.repo_id);
            if let Some(handle) = prepared.db {
                session.set_active_db(handle);
                return;
            }
            session.clear_active_db();
        }
        None => {
            session.clear_active_repo();
            session.clear_active_db();
        }
    }
}
