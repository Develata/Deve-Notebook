use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::{bootstrap_local_repo, resolve_session_repo_and_sync};
use crate::server::session::WsSession;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::node_meta;
use deve_core::models::{NodeId, NodeMeta, RepoId, RepoType};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use std::sync::Arc;

pub async fn handle_list_docs(state: &Arc<AppState>, ch: &DualChannel, session: &mut WsSession) {
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
            return ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                err.to_string(),
            ));
        }
    };

    ch.unicast(ServerMessage::RepoSwitched {
        name: repo_name.clone(),
        uuid: repo_id.to_string(),
    });

    match load_docs(state, session, &repo_name, repo_id) {
        Ok(docs) => {
            ch.unicast(ServerMessage::DocList {
                repo_id: Some(repo_id),
                docs,
            });
            match load_nodes(state, session, &repo_name, repo_id) {
                Ok(nodes) => {
                    let delta = state.tree_manager.reset_from_nodes(repo_id, nodes);
                    ch.unicast(ServerMessage::TreeUpdate {
                        repo_id: Some(repo_id),
                        delta,
                    });
                }
                Err(err) => {
                    tracing::error!("Failed to list nodes for repo {}: {:?}", repo_name, err);
                    ch.send_protocol_error(ServerError::with_detail(
                        ServerErrorCode::RequestFailed,
                        format!("Failed to list nodes: {}", err),
                    ));
                }
            }
        }
        Err(err) => {
            tracing::error!("Failed to list docs for repo {}: {:?}", repo_name, err);
            ch.send_protocol_error(ServerError::with_detail(
                ServerErrorCode::RequestFailed,
                format!("Failed to list docs: {}", err),
            ));
        }
    }
}

fn load_docs(
    state: &Arc<AppState>,
    session: &WsSession,
    repo_name: &str,
    repo_id: RepoId,
) -> anyhow::Result<Vec<(deve_core::models::DocId, String)>> {
    if let Some(handle) = session.get_active_db() {
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
    if let Some(handle) = session.get_active_db() {
        return node_meta::list_nodes(&handle.db);
    }
    if let Some(peer_id) = &session.active_branch {
        return state
            .repo
            .list_nodes(&RepoType::Remote(peer_id.clone(), repo_id));
    }
    state.repo.list_local_nodes(Some(repo_name))
}
