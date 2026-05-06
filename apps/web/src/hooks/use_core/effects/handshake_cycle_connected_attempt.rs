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

pub(super) struct ConnectedHandshakeAttemptInput<'a> {
    pub ws: &'a WsService,
    pub signals: HandshakeSignals,
    pub last_mode: &'a Rc<RefCell<Option<String>>>,
    pub handshake_attempt: &'a Rc<Cell<u64>>,
    pub maybe_mode: Option<DegradedSyncMode>,
    pub maybe_identity: Option<StoredPeerIdentity>,
    pub vector: VersionVector,
    pub repo_name: Option<String>,
    pub active_repo_id: Option<String>,
    pub branch: Option<PeerId>,
    pub current_scope_nonce: u64,
    pub should_restore: bool,
}

pub(super) fn start_connected_handshake_attempt(input: ConnectedHandshakeAttemptInput<'_>) {
    input.ws.clear_writer_ready();
    input.signals.set_handshake_ready.set(false);
    set_handshake_scope_nonce_if_changed(input.signals, None);
    if let Some(identity) = input.maybe_identity.as_ref()
        && input.maybe_mode.is_none()
        && input.active_repo_id.as_deref() != Some(identity.repo_id.as_str())
    {
        reset_scope_mismatch(
            input.last_mode,
            input.ws,
            input.signals,
            input.should_restore,
            input.repo_name.clone(),
            input.active_repo_id.clone(),
            input.branch.clone(),
        );
        return;
    }
    set_handshake_scope_nonce_if_changed(input.signals, Some(input.current_scope_nonce));
    spawn_handshake_attempt(HandshakeAttemptCtx {
        ws: input.ws.clone(),
        signals: input.signals,
        maybe_mode: input.maybe_mode,
        maybe_identity: input.maybe_identity,
        vector: input.vector,
        repo_name: input.repo_name,
        active_repo_id: input.active_repo_id,
        branch: input.branch,
        current_scope_nonce: input.current_scope_nonce,
        should_restore: input.should_restore,
        handshake_attempt: input.handshake_attempt.clone(),
        failure_last_mode: input.last_mode.clone(),
    });
}
