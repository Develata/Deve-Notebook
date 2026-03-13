use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{
    bootstrap_local_repo, map_repo_scope_error, resolve_session_repo_and_sync,
};
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{NodeId, NodeMeta, RepoId, RepoType};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub async fn handle_list_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: Option<String>,
    switch_nonce: Option<u64>,
) {
    let resolved = if session.active_branch.is_none() {
        bootstrap_local_repo(state, session).inspect(|scope| {
            session.switch_repo(scope.repo_name.clone(), Some(scope.repo_id));
        })
    } else {
        resolve_session_repo_and_sync(state, session)
    };
    let (repo_name, repo_id) = match resolved {
        Ok(scope) => (scope.repo_name, scope.repo_id),
        Err(_err) if session.active_branch.is_some() && session.active_repo.is_none() => {
            session.clear_active_repo();
            ch.send_sync_repo_unbound();
            return;
        }
        Err(err) => {
            return ch.send_protocol_error(map_repo_scope_error(err));
        }
    };

    let docs = match load_docs(state, session, repo_id) {
        Ok(docs) => docs,
        Err(err) => {
            tracing::error!("Failed to list docs for repo {}: {:?}", repo_name, err);
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to list docs: {}", err),
            ));
            return;
        }
    };
    let nodes = match load_nodes(state, session, repo_id) {
        Ok(nodes) => nodes,
        Err(err) => {
            tracing::error!("Failed to list nodes for repo {}: {:?}", repo_name, err);
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to list nodes: {}", err),
            ));
            return;
        }
    };

    ch.unicast(ServerMessage::RepoSwitched {
        branch: session.active_branch.as_ref().map(ToString::to_string),
        name: repo_name.clone(),
        uuid: repo_id.to_string(),
        switch_nonce,
    });
    ch.unicast(ServerMessage::DocList {
        request_id: request_id.clone(),
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        scope_nonce: Some(session.scope_nonce()),
        docs,
    });
    let delta = state
        .tree_manager
        .reset_from_nodes(repo_id, session.active_branch.as_ref(), nodes);
    ch.unicast(ServerMessage::TreeUpdate {
        request_id,
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        scope_nonce: Some(session.scope_nonce()),
        delta,
    });
}

fn load_docs(
    state: &Arc<AppState>,
    session: &WsSession,
    repo_id: RepoId,
) -> anyhow::Result<Vec<(deve_core::models::DocId, String)>> {
    state.repo.list_docs(&resolved_repo_type(session, repo_id))
}

fn load_nodes(
    state: &Arc<AppState>,
    session: &WsSession,
    repo_id: RepoId,
) -> anyhow::Result<Vec<(NodeId, NodeMeta)>> {
    state.repo.list_nodes(&resolved_repo_type(session, repo_id))
}

fn resolved_repo_type(session: &WsSession, repo_id: RepoId) -> RepoType {
    match session.active_branch.clone() {
        Some(peer_id) => RepoType::Remote(peer_id, repo_id),
        None => RepoType::Local(repo_id),
    }
}
