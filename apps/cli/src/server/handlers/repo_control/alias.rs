//! plan_ref:
//!   - 04_repository#host-repo-alias-contract
//!   - 07_network#repo-control-wire-contract
//!
//! SetAlias transport arm: host-local alias CAS plus exact repo-list
//! projection broadcast on committed change.

use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::ledger::HostRepoAliasError;
use deve_core::models::RepoId;
use deve_core::protocol::{
    RepoAliasBinding, RepoControlResponse, RepoReadiness, ServerErrorCode, ServerMessage,
};
use std::sync::Arc;
use uuid::Uuid;

pub(super) async fn handle_set_alias(
    state: &Arc<AppState>,
    channel: &DualChannel,
    session: &WsSession,
    request_id: Uuid,
    repo_id: RepoId,
    alias: &str,
    expected_alias_revision: u64,
) {
    let alias_runtime = state.repo.host_repo_alias_runtime();
    let watcher = state.watcher_runtime_view();
    let result = state
        .repo_mutation_gate()
        .execute_catalog_repo_unpublished(repo_id, || {
            admit_alias_mutation(watcher.repo_readiness(repo_id))?;
            alias_runtime
                .set_alias(repo_id, alias, expected_alias_revision)
                .map_err(AliasExecutionError::Alias)
        })
        .await;
    match result {
        Ok(Ok(result)) => {
            let binding = RepoAliasBinding {
                repo_id: result.binding.repo_id,
                display_alias: result.binding.alias,
                alias_revision: result.binding.alias_revision,
            };
            channel.unicast(ServerMessage::RepoControl(RepoControlResponse::AliasSet {
                request_id,
                binding,
            }));
            if result.changed {
                match crate::server::handlers::repo_list::repo_list_message(
                    state,
                    None,
                    None,
                    Some(session.scope_nonce()),
                ) {
                    Ok(message) => {
                        let _ = state.tx.send(message);
                    }
                    Err(error) => {
                        tracing::error!(%repo_id, %error, "alias committed but repo list projection failed");
                    }
                }
            }
        }
        Ok(Err(AliasExecutionError::Transitioning)) => {
            super::send_simple_error(channel, request_id, ServerErrorCode::RepoLifecycleBusy)
        }
        Ok(Err(AliasExecutionError::Alias(error))) => super::send_error(
            channel,
            request_id,
            alias_error_code(&error),
            "repo alias request failed",
            &error,
        ),
        Err(error) => super::send_error(
            channel,
            request_id,
            ServerErrorCode::RepoAliasStoreFailed,
            "repo alias mutation lane failed",
            &error,
        ),
    }
}

enum AliasExecutionError {
    Transitioning,
    Alias(HostRepoAliasError),
}

fn admit_alias_mutation(readiness: RepoReadiness) -> Result<(), AliasExecutionError> {
    if readiness == RepoReadiness::Transitioning {
        Err(AliasExecutionError::Transitioning)
    } else {
        Ok(())
    }
}

fn alias_error_code(error: &HostRepoAliasError) -> ServerErrorCode {
    match error {
        HostRepoAliasError::InvalidAlias(_) | HostRepoAliasError::UnknownLocalRepo(_) => {
            ServerErrorCode::RepoAliasInvalid
        }
        HostRepoAliasError::RevisionConflict { .. } => ServerErrorCode::RepoAliasStale,
        _ => ServerErrorCode::RepoAliasStoreFailed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alias_admission_rejects_only_lifecycle_transition() {
        assert!(matches!(
            admit_alias_mutation(RepoReadiness::Transitioning),
            Err(AliasExecutionError::Transitioning)
        ));
        for readiness in [
            RepoReadiness::Mounted,
            RepoReadiness::Readonly,
            RepoReadiness::Unavailable,
        ] {
            assert!(admit_alias_mutation(readiness).is_ok());
        }
    }
}
