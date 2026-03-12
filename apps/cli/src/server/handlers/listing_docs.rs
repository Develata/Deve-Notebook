use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{
    bootstrap_local_repo, map_repo_scope_error, resolve_session_repo_and_sync,
};
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::node_meta;
use deve_core::models::{NodeId, NodeMeta, RepoId, RepoType};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub async fn handle_list_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: Option<String>,
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

    let docs = match load_docs(state, session, &repo_name, repo_id) {
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
    let nodes = match load_nodes(state, session, &repo_name, repo_id) {
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
    });
    ch.unicast(ServerMessage::DocList {
        request_id: request_id.clone(),
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        docs,
    });
    let delta = state
        .tree_manager
        .reset_from_nodes(repo_id, session.active_branch.as_ref(), nodes);
    ch.unicast(ServerMessage::TreeUpdate {
        request_id,
        repo_id: Some(repo_id),
        branch: session.active_branch.clone(),
        delta,
    });
}

fn load_docs(
    state: &Arc<AppState>,
    session: &WsSession,
    repo_name: &str,
    repo_id: RepoId,
) -> anyhow::Result<Vec<(deve_core::models::DocId, String)>> {
    if let Some(handle) =
        session.active_db_for(session.active_branch.as_ref(), repo_name, Some(repo_id))
    {
        return deve_core::ledger::node_meta::list_file_docs(&handle.db);
    }
    if let Some(peer_id) = &session.active_branch {
        return state
            .repo
            .list_docs(&RepoType::Remote(peer_id.clone(), repo_id));
    }
    state.repo.list_local_docs(Some(repo_name))
}

fn load_nodes(
    state: &Arc<AppState>,
    session: &WsSession,
    repo_name: &str,
    repo_id: RepoId,
) -> anyhow::Result<Vec<(NodeId, NodeMeta)>> {
    if let Some(handle) =
        session.active_db_for(session.active_branch.as_ref(), repo_name, Some(repo_id))
    {
        return node_meta::list_nodes(&handle.db);
    }
    if let Some(peer_id) = &session.active_branch {
        return state
            .repo
            .list_nodes(&RepoType::Remote(peer_id.clone(), repo_id));
    }
    state.repo.list_local_nodes(Some(repo_name))
}
