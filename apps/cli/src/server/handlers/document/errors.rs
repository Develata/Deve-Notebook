//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!
//! Structured document runtime error mapping.

use crate::server::channel::DualChannel;
use crate::server::repo_scope::map_repo_scope_error;
use anyhow::anyhow;
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode};

pub(crate) struct OpenDocErrorContext<'a> {
    pub(crate) context: &'a str,
    pub(crate) scope_nonce: Option<u64>,
    pub(crate) repo_scope: &'a str,
    pub(crate) doc_id: DocId,
    pub(crate) request_id: u64,
    pub(crate) repo_id: RepoId,
    pub(crate) branch: Option<&'a PeerId>,
}

fn error_code(err: &anyhow::Error) -> ServerErrorCode {
    let detail = err.to_string();
    let lower = detail.to_ascii_lowercase();
    if lower.contains("document context invalid") || lower.contains("doc context invalid") {
        return ServerErrorCode::DocContextInvalid;
    }
    if lower.contains("document not found") || lower.contains("doc not found") {
        return ServerErrorCode::DocNotFound;
    }
    let mapped = map_repo_scope_error(anyhow!(detail.clone())).code;
    if mapped != ServerErrorCode::RequestFailed {
        return mapped;
    }
    if lower.contains("tracked document projection missing") {
        return ServerErrorCode::StoragePersistFailed;
    }
    if lower.contains("table") && lower.contains("does not exist") {
        return ServerErrorCode::StoragePersistFailed;
    }
    ServerErrorCode::RequestFailed
}

pub(crate) fn send_doc_error_with_scope_nonce(
    ch: &DualChannel,
    context: &str,
    err: anyhow::Error,
    scope_nonce: Option<u64>,
) {
    send_doc_error_with_scope_and_switch_nonce(ch, context, err, scope_nonce, None);
}

pub(crate) fn send_open_doc_error_with_scope_nonce(
    ch: &DualChannel,
    err: anyhow::Error,
    open_context: OpenDocErrorContext<'_>,
) {
    let code = error_code(&err);
    tracing::warn!(
        repo_scope = open_context.repo_scope,
        doc_id = %open_context.doc_id,
        request_id = open_context.request_id,
        repo_id = %open_context.repo_id,
        branch = ?open_context.branch,
        error_code = ?code,
        error = %err,
        context = open_context.context,
        "OpenDoc request failed"
    );
    ch.send_protocol_error_with_scope_and_switch_nonce(
        ServerError::with_detail(code, format!("{}: {}", open_context.context, err)),
        open_context.scope_nonce,
        None,
    );
}

pub(crate) fn send_doc_error_with_scope_and_switch_nonce(
    ch: &DualChannel,
    context: &str,
    err: anyhow::Error,
    scope_nonce: Option<u64>,
    switch_nonce: Option<u64>,
) {
    ch.send_protocol_error_with_scope_and_switch_nonce(
        ServerError::with_detail(error_code(&err), format!("{}: {}", context, err)),
        scope_nonce,
        switch_nonce,
    );
}

#[cfg(test)]
#[path = "errors_test.rs"]
mod tests;
