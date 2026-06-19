//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Peer merge flow orchestration.

use crate::server::session::PendingMergeConflict;
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::ledger::merge::MergeResult;
use deve_core::models::{DocId, PeerId};
use std::sync::Arc;

use super::errors;
use super::peer_apply::{MergeConflictPayload, send_merge_conflict, write_merged_content};
use super::scope::resolve_local_write_scope;

/// Invariants:
/// - 合并目标必须是当前会话解析出的本地 repo。
/// - 远端影子分支内容绝不能写回到其他 repo 的 metadata/path 映射。
pub(super) async fn handle_merge_peer(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    peer_id: String,
    doc_id: DocId,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(local_scope) = resolve_local_write_scope(state, ch, session, scope_nonce) else {
        return;
    };
    let peer_id = PeerId::new(peer_id);
    match state.repo.merge_peer_in_local_repo(
        &local_scope.repo_name,
        &peer_id,
        &local_scope.repo_id,
        doc_id,
    ) {
        Ok(MergeResult::Success(content)) => {
            session.pending_merge_conflict = None;
            write_merged_content(state, ch, &local_scope, doc_id, &content, scope_nonce);
        }
        Ok(MergeResult::Conflict {
            base,
            local,
            remote,
            conflicts,
        }) => {
            let pending = PendingMergeConflict {
                repo_id: local_scope.repo_id,
                branch: local_scope.branch.clone(),
                doc_id,
                scope_nonce,
                local_content: local.clone(),
                incoming_content: remote.clone(),
            };
            let emitted = send_merge_conflict(
                state,
                ch,
                &local_scope,
                &local_scope,
                MergeConflictPayload {
                    doc_id,
                    base,
                    local,
                    remote,
                    conflicts,
                },
                scope_nonce,
            );
            if emitted {
                session.pending_merge_conflict = Some(pending);
            }
        }
        Err(e) => errors::classified_failure(ch, format!("Merge failed: {}", e), scope_nonce),
    }
}
