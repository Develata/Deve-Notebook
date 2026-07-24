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
    use crate::server::session::WsSession;
    use crate::server::sync_hello_test_support::{build_state, unicast_channel};
    use std::time::Duration;
    use tokio::time::timeout;

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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn browser_alias_cas_returns_binding_and_publishes_same_scope_repo_list()
    -> anyhow::Result<()> {
        let (_dir, state, repo_id) = build_state()?;
        let (channel, mut unicast) = unicast_channel(&state);
        let mut broadcast = state.tx.subscribe();
        let mut session = WsSession::new();
        session.mark_browser_session();
        session.switch_repo(repo_id.to_string(), Some(repo_id));
        session.set_scope_nonce(Some(7));
        let request_id = Uuid::new_v4();
        let initial = state.repo.host_repo_alias_runtime().binding(repo_id)?;

        handle_set_alias(
            &state,
            &channel,
            &session,
            request_id,
            repo_id,
            "research",
            initial.alias_revision,
        )
        .await;

        match timeout(Duration::from_secs(2), unicast.recv()).await? {
            Some(ServerMessage::RepoControl(RepoControlResponse::AliasSet {
                request_id: actual_request,
                binding,
            })) => {
                assert_eq!(actual_request, request_id);
                assert_eq!(binding.repo_id, repo_id);
                assert_eq!(binding.display_alias, "research");
                assert_eq!(
                    binding.alias_revision,
                    initial.alias_revision.saturating_add(1)
                );
            }
            other => panic!("expected AliasSet, got {other:?}"),
        }
        match timeout(Duration::from_secs(2), broadcast.recv()).await?? {
            ServerMessage::RepoList {
                scope_nonce,
                repo_entries,
                ..
            } => {
                assert_eq!(scope_nonce, Some(7));
                let entry = repo_entries
                    .into_iter()
                    .find(|entry| entry.repo_id == repo_id)
                    .expect("renamed repo entry");
                assert_eq!(entry.display_alias, "research");
                assert_eq!(
                    entry.alias_revision,
                    initial.alias_revision.saturating_add(1)
                );
            }
            other => panic!("expected RepoList publication, got {other:?}"),
        }

        let persisted = state.repo.host_repo_alias_runtime().binding(repo_id)?;
        assert_eq!(persisted.alias, "research");
        assert_eq!(
            persisted.alias_revision,
            initial.alias_revision.saturating_add(1)
        );

        let stale_request = Uuid::new_v4();
        handle_set_alias(
            &state,
            &channel,
            &session,
            stale_request,
            repo_id,
            "stale",
            initial.alias_revision,
        )
        .await;
        match timeout(Duration::from_secs(2), unicast.recv()).await? {
            Some(ServerMessage::RepoControl(RepoControlResponse::Error { request_id, error })) => {
                assert_eq!(request_id, stale_request);
                assert_eq!(error.code, ServerErrorCode::RepoAliasStale);
                assert!(error.detail.is_none());
            }
            other => panic!("expected stale alias error, got {other:?}"),
        }
        assert!(
            broadcast.try_recv().is_err(),
            "stale CAS must not publish a repo list"
        );
        Ok(())
    }
}
