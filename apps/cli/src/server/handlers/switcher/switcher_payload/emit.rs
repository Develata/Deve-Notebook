//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Repo switch payload emission.

use crate::server::{AppState, channel::DualChannel, session::WsSession};
use std::sync::Arc;

use super::{RepoViewMessages, RepoViewPayload};

pub(crate) fn switch_scope_nonce(session: &WsSession, switch_nonce: Option<u64>) -> Option<u64> {
    session
        .is_browser_session()
        .then(|| switch_nonce.unwrap_or(session.scope_nonce()))
}

pub(crate) fn prepare_repo_view_messages(
    state: &Arc<AppState>,
    branch: Option<String>,
    request_id: Option<String>,
    scope_nonce: Option<u64>,
    switch_nonce: Option<u64>,
    repo_view: Option<RepoViewPayload>,
) -> anyhow::Result<Option<RepoViewMessages>> {
    let Some(repo_view) = repo_view else {
        return Ok(None);
    };
    let tree_branch = branch.clone().map(deve_core::models::PeerId::new);
    let delta = state.tree_manager.reset_from_nodes(
        repo_view.repo_id,
        tree_branch.as_ref(),
        repo_view.nodes,
    )?;
    Ok(Some(RepoViewMessages {
        repo_switched: deve_core::protocol::ServerMessage::RepoSwitched {
            branch: branch.clone(),
            name: repo_view.repo_name,
            uuid: repo_view.repo_id.to_string(),
            switch_nonce,
        },
        doc_list: deve_core::protocol::ServerMessage::DocList {
            request_id: request_id.clone(),
            repo_id: Some(repo_view.repo_id),
            branch: tree_branch.clone(),
            scope_nonce,
            docs: repo_view.docs,
        },
        tree_update: deve_core::protocol::ServerMessage::TreeUpdate {
            request_id,
            repo_id: Some(repo_view.repo_id),
            branch: tree_branch,
            scope_nonce,
            delta,
        },
    }))
}

pub(crate) fn emit_repo_view(ch: &DualChannel, repo_view: Option<RepoViewMessages>) {
    let Some(repo_view) = repo_view else {
        return;
    };
    ch.unicast(repo_view.repo_switched);
    ch.unicast(repo_view.doc_list);
    ch.unicast(repo_view.tree_update);
}
