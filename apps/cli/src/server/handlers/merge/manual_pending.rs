//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Manual merge pending-operation handlers.

use super::errors;
use super::manual_support::{with_engine, with_engine_mut};
use super::scope::{resolve_read_repo_id, resolve_write_repo_id};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub(super) async fn handle_get_pending_ops(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(repo_id) = resolve_read_repo_id(state, ch, session, scope_nonce) else {
        return;
    };
    let Some(pending_count) = with_engine(state, ch, repo_id, scope_nonce, |engine| {
        engine.pending_ops_count()
    }) else {
        return;
    };
    let previews = if pending_count > 0 {
        vec![(
            "(pending operations)".to_string(),
            "...".to_string(),
            "...".to_string(),
        )]
    } else {
        Vec::new()
    };
    ch.unicast(ServerMessage::PendingOpsInfo {
        request_id: Some(request_id),
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        scope_nonce,
        count: pending_count as u32,
        previews,
    });
}

pub(super) async fn handle_confirm_merge(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(repo_id) = resolve_write_repo_id(state, ch, session, scope_nonce) else {
        return;
    };
    // Pending payloads are authenticated remote-shadow input. Their engine
    // already owns the shadow merge guard; they must not acquire the local
    // authority publication gate or allocate local fact sequence numbers.
    let Some(merged) = with_engine_mut(state, ch, repo_id, scope_nonce, |engine| {
        engine.merge_pending()
    }) else {
        return;
    };
    match merged {
        Ok(count) => {
            tracing::info!("Merged {} pending operations", count);
            let _ = state.tx.send(ServerMessage::MergeComplete {
                repo_id: Some(repo_id),
                branch: session.active_branch.clone(),
                scope_nonce,
                merged_count: count as u32,
            });
        }
        Err(e) => {
            tracing::error!("Merge failed: {:?}", e);
            errors::classified_failure(ch, format!("Merge failed: {}", e), scope_nonce);
        }
    }
}

pub(super) async fn handle_discard_pending(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let Some(repo_id) = resolve_write_repo_id(state, ch, session, scope_nonce) else {
        return;
    };
    if with_engine_mut(state, ch, repo_id, scope_nonce, |engine| {
        engine.clear_pending();
    })
    .is_none()
    {
        return;
    };
    tracing::info!("Discarded all pending operations");
    ch.unicast(ServerMessage::PendingDiscarded {
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        scope_nonce,
    });
}
