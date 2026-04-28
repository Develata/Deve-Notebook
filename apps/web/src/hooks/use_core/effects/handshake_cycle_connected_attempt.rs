//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::hooks::use_core::types::HandshakeSignals;
use crate::storage::DegradedSyncMode;
use crate::storage::identity::StoredPeerIdentity;
use deve_core::models::{PeerId, VersionVector};
use leptos::prelude::Set;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::super::super::handshake_reset::reset_scope_mismatch;
use super::super::super::handshake_send::{HandshakeAttemptCtx, spawn_handshake_attempt};
use super::super::super::handshake_state::set_handshake_scope_nonce_if_changed;

#[allow(clippy::too_many_arguments)]
pub(super) fn start_connected_handshake_attempt(
    ws: &WsService,
    signals: HandshakeSignals,
    last_mode: &Rc<RefCell<Option<String>>>,
    handshake_attempt: &Rc<Cell<u64>>,
    maybe_mode: Option<DegradedSyncMode>,
    maybe_identity: Option<StoredPeerIdentity>,
    vector: VersionVector,
    repo_name: Option<String>,
    active_repo_id: Option<String>,
    branch: Option<PeerId>,
    current_scope_nonce: u64,
    should_restore: bool,
) {
    ws.clear_writer_ready();
    signals.set_handshake_ready.set(false);
    set_handshake_scope_nonce_if_changed(signals, None);
    if let Some(identity) = maybe_identity.as_ref()
        && maybe_mode.is_none()
        && active_repo_id.as_deref() != Some(identity.repo_id.as_str())
    {
        reset_scope_mismatch(
            last_mode,
            ws,
            signals,
            should_restore,
            repo_name.clone(),
            active_repo_id.clone(),
            branch.clone(),
        );
        return;
    }
    set_handshake_scope_nonce_if_changed(signals, Some(current_scope_nonce));
    spawn_handshake_attempt(HandshakeAttemptCtx {
        ws: ws.clone(),
        signals,
        maybe_mode,
        maybe_identity,
        vector,
        repo_name,
        active_repo_id,
        branch,
        current_scope_nonce,
        should_restore,
        handshake_attempt: handshake_attempt.clone(),
        failure_last_mode: last_mode.clone(),
    });
}
