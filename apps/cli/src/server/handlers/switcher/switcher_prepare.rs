//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Repo switch preparation and database handle resolution.

use crate::server::AppState;
use crate::server::repo_mutation::RepoMutationGateError;
use crate::server::session::WsSession;
use deve_core::ledger::CatalogMembershipToken;
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::RepoId;
use std::sync::Arc;

mod branch;
mod recovery;

pub(super) struct PreparedRepoSwitch {
    /// Canonical execution selector used for storage/runtime operations.
    pub repo_name: String,
    /// Backend-verified display label used for session and wire payloads.
    pub session_name: String,
    pub repo_id: Option<RepoId>,
    pub db: Option<DatabaseHandle>,
    pub degraded_docs_only: bool,
    pub catalog_membership: Option<CatalogMembershipToken>,
}

pub(crate) use branch::validate_branch_target;

pub(super) async fn prepare_repo_switch(
    state: &Arc<AppState>,
    branch: Option<&deve_core::models::PeerId>,
    repo_name: String,
) -> anyhow::Result<PreparedRepoSwitch> {
    let repo_info = state.repo.get_repo_info_for(branch, Some(&repo_name))?;
    let Some(repo_info) = repo_info else {
        let scope = if branch.is_some() { "Remote" } else { "Local" };
        return Err(anyhow::anyhow!(
            "{scope} repository UUID not resolved for selector: {}",
            repo_name
        ));
    };
    let repo_id = Some(repo_info.uuid);
    let session_name = repo_info.name.clone();
    if branch.is_some() {
        let handle = state.repo.open_database(branch, &repo_name)?;
        return Ok(PreparedRepoSwitch {
            repo_name,
            session_name,
            repo_id,
            db: Some(handle),
            degraded_docs_only: false,
            catalog_membership: None,
        });
    }
    let prepared = state
        .sync_manager
        .prepare_local_repo_materialization(&repo_name)?;
    let execution = state
        .repo_mutation_gate()
        .execute_mounted_repo_unpublished(repo_info.uuid, || {
            let bound_name = match state
                .repo
                .resolve_local_repo_name_for_execution(Some(repo_info.uuid), Some(&repo_name))
            {
                Ok(name) => name,
                Err(error) => return Err(error),
            };
            state
                .sync_manager
                .apply_prepared_local_repo_materialization(&bound_name, prepared)
        })
        .await;
    let degraded_docs_only = match execution {
        Ok(Ok(())) => false,
        Ok(Err(err)) if recovery::should_degrade_local_projection(&err) => {
            tracing::warn!(
                repo_name = %repo_name,
                error = %err,
                "Local repo switch degraded to docs-only fallback due to broken structure projection"
            );
            true
        }
        Ok(Err(err)) => return Err(err),
        Err(RepoMutationGateError::WorkspaceIngestionUnavailable) => {
            tracing::warn!(
                repo_name = %repo_name,
                "Skipping local projection materialization because workspace ingestion is unavailable"
            );
            false
        }
        Err(error) => return Err(anyhow::Error::new(error)),
    };
    let catalog_membership = state.catalog_membership_runtime().issue(repo_info.uuid)?;
    Ok(PreparedRepoSwitch {
        repo_name,
        session_name,
        repo_id,
        db: None,
        degraded_docs_only,
        catalog_membership: Some(catalog_membership),
    })
}

pub(super) fn commit_session_switch(
    state: &Arc<AppState>,
    session: &mut WsSession,
    branch: Option<String>,
    prepared: Option<PreparedRepoSwitch>,
    scope_nonce: Option<u64>,
) -> anyhow::Result<()> {
    if let Some(prepared) = prepared.as_ref()
        && prepared.db.is_none()
    {
        let membership = prepared
            .catalog_membership
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("local repo switch is missing catalog membership"))?;
        state.catalog_membership_runtime().revalidate(membership)?;
    }
    session.set_scope_nonce(scope_nonce);
    session.clear_sync_binding();
    session.switch_branch(branch);
    match prepared {
        Some(prepared) => {
            session.switch_repo(prepared.session_name, prepared.repo_id);
            if let Some(membership) = prepared.catalog_membership {
                session.bind_catalog_membership(membership);
            }
            if let Some(handle) = prepared.db {
                session.set_active_db(handle);
                return Ok(());
            }
            session.clear_active_db();
        }
        None => {
            session.clear_active_repo();
            session.clear_active_db();
        }
    }
    Ok(())
}
