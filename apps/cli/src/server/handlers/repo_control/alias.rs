//! plan_ref:
//!   - 04_repository#host-repo-alias-contract
//!   - 07_network#repo-control-wire-contract
//!
//! SetAlias transport arm: host-local alias CAS plus exact repo-list
//! projection broadcast on committed change.

use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::ledger::HostRepoAliasError;
use deve_core::models::RepoId;
use deve_core::protocol::{RepoAliasBinding, RepoControlResponse, ServerErrorCode, ServerMessage};
use std::sync::Arc;
use uuid::Uuid;

pub(super) fn handle_set_alias(
    state: &Arc<AppState>,
    channel: &DualChannel,
    session: &WsSession,
    request_id: Uuid,
    repo_id: RepoId,
    alias: &str,
    expected_alias_revision: u64,
) {
    let result =
        state
            .repo
            .host_repo_alias_runtime()
            .set_alias(repo_id, alias, expected_alias_revision);
    match result {
        Ok(result) => {
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
        Err(error) => super::send_error(
            channel,
            request_id,
            alias_error_code(&error),
            "repo alias request failed",
            &error,
        ),
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
