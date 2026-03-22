//! # Docs 视图刷新辅助逻辑
//!
//! Invariants:
//! - Docs handler 不得直接把 `TreeManager` 当权威写源。
//! - `DocList + TreeUpdate` 必须来自当前 repo 的 node/content projection。

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::ResolvedRepo;
use crate::server::session::WsSession;
use anyhow::Result;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{RepoType, StructureOp};
use deve_core::protocol::ServerMessage;
use std::sync::Arc;

pub fn broadcast_local_projection_refresh(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    scope: &ResolvedRepo,
) -> Result<()> {
    let repo = RepoType::Local(scope.repo_id);
    let docs = state.repo.list_docs(&repo)?;
    let nodes = state.repo.list_nodes(&repo)?;
    let delta = state
        .tree_manager
        .reset_from_nodes(scope.repo_id, None, nodes)?;
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());

    ch.unicast(ServerMessage::DocList {
        request_id: None,
        repo_id: Some(scope.repo_id),
        branch: None,
        scope_nonce,
        docs,
    });
    ch.unicast(ServerMessage::TreeUpdate {
        request_id: None,
        repo_id: Some(scope.repo_id),
        branch: None,
        scope_nonce,
        delta,
    });
    Ok(())
}

/// 增量广播 TreeDelta，避免全量 projection refresh。
///
/// 将 StructureOp 映射为 TreeDelta::Add/Remove/Update，
/// 逐条发送给前端。同时刷新 DocList 以保持一致。
pub fn broadcast_incremental_tree_deltas(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &WsSession,
    scope: &ResolvedRepo,
    ops: &[StructureOp],
) -> Result<()> {
    let deltas = state
        .tree_manager
        .apply_structure_ops(scope.repo_id, None, ops)?;
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let docs = state.repo.list_docs(&RepoType::Local(scope.repo_id))?;
    ch.unicast(ServerMessage::DocList {
        request_id: None,
        repo_id: Some(scope.repo_id),
        branch: None,
        scope_nonce,
        docs,
    });
    for delta in deltas {
        ch.unicast(ServerMessage::TreeUpdate {
            request_id: None,
            repo_id: Some(scope.repo_id),
            branch: None,
            scope_nonce,
            delta,
        });
    }
    Ok(())
}
