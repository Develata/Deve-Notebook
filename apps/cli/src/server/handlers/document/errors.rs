use crate::server::channel::DualChannel;
use crate::server::repo_scope::map_repo_scope_error;
use anyhow::anyhow;
use deve_core::models::{DocId, PeerId, RepoId};
use deve_core::protocol::{ServerError, ServerErrorCode};

fn error_code(err: &anyhow::Error) -> ServerErrorCode {
    let detail = err.to_string();
    let mapped = map_repo_scope_error(anyhow!(detail.clone())).code;
    if mapped != ServerErrorCode::RequestFailed {
        return mapped;
    }
    if detail
        .to_ascii_lowercase()
        .contains("tracked document projection missing")
    {
        return ServerErrorCode::StoragePersistFailed;
    }
    if detail.to_ascii_lowercase().contains("table")
        && detail.to_ascii_lowercase().contains("does not exist")
    {
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
    context: &str,
    err: anyhow::Error,
    scope_nonce: Option<u64>,
    repo_scope: &str,
    doc_id: DocId,
    request_id: u64,
    repo_id: RepoId,
    branch: Option<&PeerId>,
) {
    let code = error_code(&err);
    tracing::warn!(
        repo_scope,
        doc_id = %doc_id,
        request_id,
        repo_id = %repo_id,
        branch = ?branch,
        error_code = ?code,
        error = %err,
        context,
        "OpenDoc request failed"
    );
    ch.send_protocol_error_with_scope_and_switch_nonce(
        ServerError::with_detail(code, format!("{}: {}", context, err)),
        scope_nonce,
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
