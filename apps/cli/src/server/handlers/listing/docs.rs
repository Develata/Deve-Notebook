//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime
//!   - 06_repository#tree-projection-contract

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::handlers::document::errors::send_doc_error_with_scope_and_switch_nonce;
use crate::server::handlers::switcher::{
    RepoViewPayload, emit_repo_view, prepare_repo_view_messages, switch_scope_nonce,
};
use crate::server::repo_scope::resolve_session_repo_or_bootstrap_local;
use crate::server::session::WsSession;
use std::sync::Arc;

mod scope;

use self::scope::LocalBootstrapGuard;
use super::scope::map_listing_repo_scope_error;

pub async fn handle_list_docs(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: Option<String>,
    switch_nonce: Option<u64>,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let local_bootstrap_guard = LocalBootstrapGuard::new(session);
    if scope::precheck_remote_scope(state, ch, session, scope_nonce, switch_nonce) {
        return;
    }
    let resolved = resolve_session_repo_or_bootstrap_local(state, session);
    let (repo_name, repo_id) = match resolved {
        Ok(scope) => (scope.repo_name, scope.repo_id),
        Err(err) => {
            return ch.send_protocol_error_with_scope_and_switch_nonce(
                map_listing_repo_scope_error(err),
                scope_nonce,
                switch_nonce,
            );
        }
    };

    let docs = match scope::load_docs(state, session, repo_id) {
        Ok(docs) => docs,
        Err(err) => {
            local_bootstrap_guard.rollback_after_error(session);
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
    let nodes = match scope::load_nodes(state, session, repo_id) {
        Ok(nodes) => nodes,
        Err(err) => {
            local_bootstrap_guard.rollback_after_error(session);
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
            local_bootstrap_guard.rollback_after_error(session);
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
