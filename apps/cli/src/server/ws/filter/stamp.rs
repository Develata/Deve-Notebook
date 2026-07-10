//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! Scope nonce stamping for runtime broadcasts.

use deve_core::protocol::{ServerError, ServerMessage};

pub(super) fn stamp_scope_nonce(msg: ServerMessage, scope_nonce: u64) -> ServerMessage {
    match msg {
        ServerMessage::FsChangeDetected {
            repo_id,
            branch,
            path,
            change_type,
            has_conflict,
            scope_nonce: _,
        } => ServerMessage::FsChangeDetected {
            repo_id,
            branch,
            scope_nonce: Some(scope_nonce),
            path,
            change_type,
            has_conflict,
        },
        ServerMessage::CommitAck {
            repo_id,
            branch,
            commit_id,
            timestamp,
            scope_nonce: _,
        } => ServerMessage::CommitAck {
            repo_id,
            branch,
            scope_nonce: Some(scope_nonce),
            commit_id,
            timestamp,
        },
        ServerMessage::NewOp {
            repo_id,
            branch,
            doc_id,
            entry,
            scope_nonce: _,
        } => ServerMessage::NewOp {
            repo_id,
            branch,
            scope_nonce: Some(scope_nonce),
            doc_id,
            entry,
        },
        ServerMessage::MergeComplete {
            repo_id,
            branch,
            merged_count,
            scope_nonce: _,
        } => ServerMessage::MergeComplete {
            repo_id,
            branch,
            scope_nonce: Some(scope_nonce),
            merged_count,
        },
        ServerMessage::PeerDeleted { peer_id, .. } => ServerMessage::PeerDeleted {
            peer_id,
            scope_nonce: Some(scope_nonce),
        },
        other => other,
    }
}

pub(super) fn scoped_protocol_error(
    error: ServerError,
    switch_nonce: Option<u64>,
    scope_nonce: Option<u64>,
) -> ServerMessage {
    ServerMessage::ProtocolError {
        error,
        switch_nonce,
        scope_nonce,
    }
}
