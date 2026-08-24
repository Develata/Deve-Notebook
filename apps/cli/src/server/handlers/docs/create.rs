// apps/cli/src/server/handlers/docs/create.rs
//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#tree-projection-contract
//!   - 03_storage/index#internal-path-normalization
//!   - 09_web_thin_client_ledger#document-create-intent
//!
//! # 创建文档处理器

use super::create_file::handle_file_create;
use super::create_folder::handle_folder_create;
use super::errors;
use super::path_validation::{validate_create_file_path, validate_create_folder_path};
use super::{normalize_repo_path_input, resolve_local_write_scope_result};
use crate::server::AppState;
use crate::server::channel::DualChannel;
use crate::server::repo_scope::local_repo_path;
use crate::server::session::WsSession;
use deve_core::models::{DocId, NodeId};
use deve_core::protocol::{
    DocumentCreateProjectionOutcome, DocumentCreateRequest, DocumentCreateResponse,
    DocumentCreateResponseContext, ServerError, ServerErrorCode, ServerMessage,
};
use std::sync::Arc;

#[cfg(test)]
mod tests;

/// 处理创建文档请求
///
/// **流程**:
/// 1. 校验文件名 (防止遍历攻击、深度超限)
/// 2. 确保父目录存在
/// 3. 创建文件并写入默认内容
/// 4. 在 Ledger 中注册 DocId
/// 5. 更新 TreeManager 并广播 TreeDelta
pub async fn handle_create_doc_request(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request: DocumentCreateRequest,
) {
    let context = DocumentCreateResponseContext::from(&request);
    if request.scope_nonce.get() != session.scope_nonce() {
        reject(ch, context, ServerErrorCode::ScStaleScope);
        return;
    }
    if request.branch.is_some() || session.active_branch.is_some() {
        reject(ch, context, ServerErrorCode::ScRemoteBranchReadonly);
        return;
    }
    let scope = match resolve_local_write_scope_result(state, session) {
        Ok(scope) => scope,
        Err(error) => {
            reject_error(ch, context, error);
            return;
        }
    };
    if scope.repo_id != request.repo_id || scope.branch != request.branch {
        reject(ch, context, ServerErrorCode::ScStaleScope);
        return;
    }

    let Some(filename) = normalize_name(request.path) else {
        reject(ch, context, ServerErrorCode::RequestFailed);
        return;
    };

    let validation = if filename.ends_with('/') {
        validate_create_folder_path(&filename)
    } else {
        validate_create_file_path(&filename)
    };
    if let Err(error) = validation {
        tracing::error!(detail = %error.detail, "Document Create path rejected");
        reject(ch, context, ServerErrorCode::RequestFailed);
        return;
    }

    let path = match local_repo_path(state, &scope, &filename) {
        Ok(path) => path,
        Err(err) => {
            tracing::error!(error = ?err, "Document Create path resolution failed");
            reject(ch, context, ServerErrorCode::RequestFailed);
            return;
        }
    };

    let result = if filename.ends_with('/') {
        handle_folder_create(state, &scope, &path, &filename, request.proposed_node_id).await
    } else {
        handle_file_create(state, &scope, &path, &filename, request.proposed_node_id).await
    };
    match result {
        Ok(receipt) => ch.unicast(ServerMessage::DocumentCreate(
            DocumentCreateResponse::Created {
                context,
                node_id: receipt.node_id,
                doc_id: receipt.doc_id,
                path: filename,
                projection_outcome: receipt.projection_outcome,
            },
        )),
        Err(error) => reject_error(ch, context, error),
    }
}

#[cfg(test)]
pub async fn handle_create_doc(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    path: String,
) {
    let repo_id = session.active_repo_id.or_else(|| {
        state
            .repo
            .get_repo_info()
            .ok()
            .flatten()
            .map(|info| info.uuid)
    });
    let Some(repo_id) = repo_id else {
        ch.unicast(ServerMessage::DocumentCreate(
            DocumentCreateResponse::Rejected {
                context: DocumentCreateResponseContext {
                    proposed_node_id: NodeId::new(),
                    repo_id: deve_core::models::RepoId::nil(),
                    branch: None,
                    scope_nonce: deve_core::protocol::ScopeNonce::new(session.scope_nonce()),
                },
                error: ServerError::new(ServerErrorCode::ScRepoNotSelected),
            },
        ));
        return;
    };
    let request = DocumentCreateRequest {
        proposed_node_id: NodeId::new(),
        repo_id,
        branch: session.active_branch.clone(),
        scope_nonce: deve_core::protocol::ScopeNonce::new(session.scope_nonce()),
        path,
    };
    handle_create_doc_request(state, ch, session, request).await;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct DocumentCreateReceipt {
    pub node_id: NodeId,
    pub doc_id: Option<DocId>,
    pub projection_outcome: DocumentCreateProjectionOutcome,
}

pub(super) type DocumentCreateResult = Result<DocumentCreateReceipt, ServerError>;

#[derive(Debug, thiserror::Error)]
#[error("Document Create conflicts with an existing authority identity or target")]
pub(super) struct DocumentCreateConflict;

pub(super) fn storage_conflict() -> anyhow::Error {
    anyhow::Error::new(DocumentCreateConflict)
}

pub(super) fn classify_execution_error(error: anyhow::Error) -> ServerError {
    if error.downcast_ref::<DocumentCreateConflict>().is_some() {
        ServerError::new(ServerErrorCode::StorageConflict)
    } else {
        errors::classified_error(error.to_string())
    }
}

fn reject(ch: &DualChannel, context: DocumentCreateResponseContext, code: ServerErrorCode) {
    reject_error(ch, context, ServerError::new(code));
}

fn reject_error(ch: &DualChannel, context: DocumentCreateResponseContext, error: ServerError) {
    ch.unicast(ServerMessage::DocumentCreate(
        DocumentCreateResponse::Rejected {
            context,
            error: ServerError::new(error.code),
        },
    ));
}

fn normalize_name(name: String) -> Option<String> {
    let mut name = normalize_repo_path_input(&name)?;
    if !name.ends_with('/') && !name.ends_with(".md") {
        name.push_str(".md");
    }
    Some(name)
}
