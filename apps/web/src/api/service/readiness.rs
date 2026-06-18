//! plan_ref:
//!   - 08_auth#unauthorized-handling
//!   - 08_auth#unauthorized-disconnected-ui
//!   - 09_web_thin_client_ledger#write-readiness
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!

use deve_core::native_adapter::NativeRuntimeReadiness;

use super::ConnectionStatus;

#[derive(Clone, Debug)]
pub(super) struct NativeRuntimeConnectionState {
    pub status: ConnectionStatus,
    pub node_role: String,
    pub node_role_probe_failed: bool,
    pub ready_repo_id: Option<String>,
    pub ready_scope_nonce: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct NativeRuntimeReadinessTarget<'a> {
    pub repo_id: Option<&'a str>,
    pub scope_nonce: Option<u64>,
    pub handshake_ready: bool,
}

pub(crate) fn is_current_connection_message(message_epoch: u64, current_epoch: u64) -> bool {
    message_epoch == current_epoch
}

pub(super) fn writer_ready_matches(
    ready_repo_id: Option<&str>,
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

fn scope_nonce_current_matches(
    ready_repo_id: Option<&str>,
    ready_scope_nonce: Option<u64>,
    repo_id: Option<&str>,
    scope_nonce: Option<u64>,
) -> bool {
    writer_ready_matches(ready_repo_id, ready_scope_nonce, repo_id, scope_nonce)
}

pub(super) fn native_runtime_readiness_from_parts(
    state: NativeRuntimeConnectionState,
    target: NativeRuntimeReadinessTarget<'_>,
) -> NativeRuntimeReadiness {
    NativeRuntimeReadiness {
        endpoint_reachable: state.status == ConnectionStatus::Connected,
        auth_status_valid: !matches!(
            state.status,
            ConnectionStatus::Unauthorized
                | ConnectionStatus::NativeBootstrapInvalid
                | ConnectionStatus::NativeSessionPending
        ),
        node_role_readable: !state.node_role_probe_failed && !state.node_role.trim().is_empty(),
        repo_handshake_complete: target.handshake_ready,
        writer_ready: writer_ready_matches(
            state.ready_repo_id.as_deref(),
            state.ready_scope_nonce,
            target.repo_id,
            target.scope_nonce,
        ),
        scope_nonce_current: scope_nonce_current_matches(
            state.ready_repo_id.as_deref(),
            state.ready_scope_nonce,
            target.repo_id,
            target.scope_nonce,
        ),
    }
}
