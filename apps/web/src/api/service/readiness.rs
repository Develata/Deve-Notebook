//! plan_ref:
//!   - 09_auth#unauthorized-handling
//!   - 09_auth#unauthorized-disconnected-ui
//!

use deve_core::native_adapter::NativeRuntimeReadiness;

use super::ConnectionStatus;

pub(crate) fn is_current_connection_message(message_epoch: u64, current_epoch: u64) -> bool {
    message_epoch == current_epoch
}

pub(super) fn writer_ready_matches(
    ready_repo_id: Option<String>,
    ready_scope_nonce: Option<u64>,
    repo_id: Option<&str>,
    scope_nonce: Option<u64>,
) -> bool {
    match (ready_repo_id, ready_scope_nonce, repo_id, scope_nonce) {
        (Some(ready_repo_id), Some(ready_scope_nonce), Some(repo_id), Some(scope_nonce)) => {
            ready_repo_id == repo_id && ready_scope_nonce == scope_nonce
        }
        _ => false,
    }
}

pub(super) fn native_runtime_readiness_from_parts(
    status: ConnectionStatus,
    node_role: String,
    node_role_probe_failed: bool,
    ready_repo_id: Option<String>,
    ready_scope_nonce: Option<u64>,
    repo_id: Option<&str>,
    scope_nonce: Option<u64>,
    handshake_ready: bool,
) -> NativeRuntimeReadiness {
    NativeRuntimeReadiness {
        endpoint_reachable: status == ConnectionStatus::Connected,
        auth_status_valid: !matches!(
            status,
            ConnectionStatus::Unauthorized
                | ConnectionStatus::NativeBootstrapInvalid
                | ConnectionStatus::NativeSessionPending
        ),
        node_role_readable: !node_role_probe_failed && !node_role.trim().is_empty(),
        repo_handshake_complete: handshake_ready,
        writer_ready: writer_ready_matches(ready_repo_id, ready_scope_nonce, repo_id, scope_nonce),
        scope_nonce_current: matches!(
            (ready_scope_nonce, scope_nonce),
            (Some(ready_scope_nonce), Some(scope_nonce)) if ready_scope_nonce == scope_nonce
        ),
    }
}
