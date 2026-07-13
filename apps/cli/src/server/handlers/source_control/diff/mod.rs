//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
//! Source-control document diff dispatch.

mod remote;
mod remote_content;
#[cfg(test)]
mod remote_test;
#[cfg(test)]
mod remote_test_extra;
#[cfg(test)]
mod remote_test_support;

use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::session::WsSession;
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::{ScPathTarget, ScopeNonce};
use std::sync::Arc;

/// 获取文档的 Diff。
///
/// **Local 分支**: 已提交版本 (左) vs 当前版本 (右)
/// **Remote 分支**: Local 对应文档 (左) vs Remote 文档 (右)
pub async fn handle_get_doc_diff(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    target: ScPathTarget,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    if session.active_branch.is_some() {
        remote::handle_remote_diff(state, ch, session, request_id, target).await;
        return;
    }
    let scope = match super::repo_scope::resolve_current_local_repo(state, session) {
        Ok(scope) => scope,
        Err(e) => return super::errors::send_ws_scoped(ch, e, scope_nonce),
    };
    let (doc_id, normalized, old_content, new_content) = match state
        .repo
        .doc_diff_payload_for_target_in_local_repo(&scope.repo_name, &target)
    {
        Ok(payload) => payload,
        Err(e) => {
            return super::errors::send_ws_scoped(
                ch,
                super::errors::map_repo_error(super::errors::ScOp::DiffDoc(target.path.clone()), e),
                scope_nonce,
            );
        }
    };

    spawn_document_projection(
        state,
        ch,
        session,
        DocumentProjectionRequest {
            request_id,
            repo_id: scope.repo_id,
            branch: scope.branch,
            scope_nonce: ScopeNonce::new(scope_nonce.unwrap_or_default()),
            doc_id,
            path: normalized,
            base_content: old_content,
            target_content: new_content,
        },
    );
}

pub async fn handle_compute_diff_projection(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request_id: String,
    revision: u64,
    base_content: String,
    target_content: String,
) {
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope = match super::repo_scope::resolve_current_repo_scope(state, session) {
        Ok(scope) => scope,
        Err(error) => return super::errors::send_ws_scoped(ch, error, scope_nonce),
    };
    let nonce = ScopeNonce::new(scope_nonce.unwrap_or_default());
    let Some(ticket) = session.diff_projection_jobs.begin_draft(
        request_id,
        revision,
        scope.repo_id,
        scope.branch,
        nonce,
    ) else {
        return;
    };
    state.diff_projection_executor().spawn(
        ticket,
        base_content,
        target_content,
        crate::server::diff_projection::DiffJobResponse::Draft,
        ch.clone(),
    );
}

pub(super) struct DocumentProjectionRequest {
    request_id: String,
    repo_id: RepoId,
    branch: Option<PeerId>,
    scope_nonce: ScopeNonce,
    doc_id: Option<DocId>,
    path: String,
    base_content: String,
    target_content: String,
}

pub(super) fn spawn_document_projection(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request: DocumentProjectionRequest,
) {
    let ticket = session.diff_projection_jobs.begin_fixed(
        request.request_id,
        request.repo_id,
        request.branch,
        request.scope_nonce,
    );
    state.diff_projection_executor().spawn(
        ticket,
        request.base_content,
        request.target_content,
        crate::server::diff_projection::DiffJobResponse::Document {
            doc_id: request.doc_id,
            path: request.path,
        },
        ch.clone(),
    );
}
