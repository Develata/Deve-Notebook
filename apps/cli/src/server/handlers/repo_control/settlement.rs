//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 04_repository#repo-scope-runtime
//!
//! Transport-side application of settled lifecycle publications: exact scope
//! validation, fallback switch, and the two-frame no-scope sequence.

use crate::server::runtime::repo_lifecycle_job_runtime::RepoLifecycleSettledPublication;
use crate::server::runtime::repo_session_runtime::{
    FinalRepoListProjection, enqueue_no_scope_sequence,
};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use std::sync::Arc;
use uuid::Uuid;

use super::send_simple_error;

#[allow(clippy::too_many_arguments)] // Exact settlement identity stays explicit at the transport adapter.
pub(crate) async fn apply_lifecycle_settlement(
    state: &Arc<AppState>,
    channel: &DualChannel,
    session: &mut WsSession,
    request_id: Uuid,
    expected_scope_nonce: u64,
    switch_nonce: u64,
    publication: RepoLifecycleSettledPublication,
    final_list: FinalRepoListProjection,
) -> bool {
    if !session.is_browser_session() || session.scope_nonce() != expected_scope_nonce {
        return true;
    }
    match publication {
        RepoLifecycleSettledPublication::Created { repo_id, mounted } => {
            if !mounted {
                channel.unicast(ServerMessage::RepoList {
                    request_id: None,
                    branch: None,
                    scope_nonce: Some(session.scope_nonce()),
                    repo_entries: final_list.entries,
                });
                send_simple_error(
                    channel,
                    request_id,
                    ServerErrorCode::RepoLifecycleCommittedPartial,
                );
                return true;
            }
            crate::server::handlers::switcher::handle_switch_repo(
                state,
                channel,
                session,
                repo_id.to_string(),
                Some(repo_id),
                Some(switch_nonce),
            )
            .await;
            true
        }
        RepoLifecycleSettledPublication::Removed {
            repo_id,
            fallback_repo_id,
        } => {
            if session.active_repo_id != Some(repo_id) {
                channel.unicast(ServerMessage::RepoList {
                    request_id: None,
                    branch: None,
                    scope_nonce: Some(session.scope_nonce()),
                    repo_entries: final_list.entries,
                });
                return true;
            }
            if let Some(fallback_repo_id) = fallback_repo_id {
                crate::server::handlers::switcher::handle_switch_repo(
                    state,
                    channel,
                    session,
                    fallback_repo_id.to_string(),
                    Some(fallback_repo_id),
                    Some(switch_nonce),
                )
                .await;
                if session.active_repo_id == Some(fallback_repo_id) {
                    return true;
                }
            }
            state.revoke_source_control_write_grant_for_session(session);
            session.commit_no_scope(repo_id, switch_nonce);
            enqueue_no_scope_sequence(channel, final_list, switch_nonce, Some(switch_nonce));
            true
        }
    }
}
