use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::document::errors::send_doc_error_with_scope_and_switch_nonce;
use crate::server::handlers::switcher::{
    RepoViewPayload, emit_repo_view, prepare_repo_view_messages, switch_scope_nonce,
};
use crate::server::repo_scope::{
    bootstrap_local_repo, map_repo_scope_error, resolve_session_repo_and_sync,
    stale_unbound_remote_scope_detail,
};
use crate::server::session::WsSession;
use crate::server::shadow_scope;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{NodeId, NodeMeta, RepoId, RepoType};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub async fn handle_list_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: Option<String>,
    switch_nonce: Option<u64>,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    if let Some(branch) = session.active_branch.as_ref().cloned()
        && session.active_repo.is_none()
        && session.active_repo_id.is_none()
    {
        if let Err(error) = shadow_scope::map_remote_branch_availability(state, &branch) {
            if shadow_scope::should_clear_missing_remote_branch(&error) {
                shadow_scope::clear_stale_remote_branch(session);
            } else {
                session.clear_active_db();
                session.clear_sync_binding();
            }
            ch.send_protocol_error_with_scope_and_switch_nonce(error, scope_nonce, switch_nonce);
            return;
        }
        if session.has_runtime_scope_binding() {
            session.clear_active_db();
            session.clear_sync_binding();
            ch.send_protocol_error_with_scope_and_switch_nonce(
                ServerError::with_detail(
                    ServerErrorCode::ScRepoContextInvalid,
                    stale_unbound_remote_scope_detail(&branch),
                ),
                scope_nonce,
                switch_nonce,
            );
            return;
        }
    }
    let resolved = if session.active_branch.is_none()
        && session.active_repo.is_none()
        && session.active_repo_id.is_none()
        && !session.has_runtime_scope_binding()
    {
        bootstrap_local_repo(state, session)
    } else {
        resolve_session_repo_and_sync(state, session)
    };
    let (repo_name, repo_id) = match resolved {
        Ok(scope) => (scope.repo_name, scope.repo_id),
        Err(err) => {
            return ch.send_protocol_error_with_scope_and_switch_nonce(
                map_repo_scope_error(err),
                scope_nonce,
                switch_nonce,
            );
        }
    };

    let docs = match load_docs(state, session, repo_id) {
        Ok(docs) => docs,
        Err(err) => {
            tracing::error!("Failed to list docs for repo {}: {:?}", repo_name, err);
            send_doc_error_with_scope_and_switch_nonce(
                ch,
                "Failed to list docs",
                err,
                scope_nonce,
                switch_nonce,
            );
            return;
        }
    };
    let nodes = match load_nodes(state, session, repo_id) {
        Ok(nodes) => nodes,
        Err(err) => {
            tracing::error!("Failed to list nodes for repo {}: {:?}", repo_name, err);
            send_doc_error_with_scope_and_switch_nonce(
                ch,
                "Failed to list nodes",
                err,
                scope_nonce,
                switch_nonce,
            );
            return;
        }
    };
    let branch = session.active_branch.as_ref().map(ToString::to_string);
    let repo_view = match prepare_repo_view_messages(
        state,
        branch,
        request_id,
        switch_scope_nonce(session, switch_nonce),
        switch_nonce,
        Some(RepoViewPayload {
            repo_name: repo_name.clone(),
            repo_id,
            docs,
            nodes,
        }),
    ) {
        Ok(repo_view) => repo_view,
        Err(err) => {
            send_doc_error_with_scope_and_switch_nonce(
                ch,
                "Failed to rebuild tree projection",
                err,
                scope_nonce,
                switch_nonce,
            );
            return;
        }
    };
    if session.active_branch.is_none()
        && (session.active_repo.as_deref() != Some(repo_name.as_str())
            || session.active_repo_id != Some(repo_id))
    {
        session.switch_repo(repo_name, Some(repo_id));
    }
    emit_repo_view(ch, repo_view);
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
