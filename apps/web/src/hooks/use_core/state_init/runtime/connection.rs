//! plan_ref:
//!   - 07_network#web-ws-runtime
//!
use deve_core::models::PeerId;
use leptos::prelude::*;
use std::collections::HashMap;

use super::super::super::navigation::PendingNavigation;
use super::super::super::types::PeerSession;
use crate::runtime::document::pending::PendingLocalEdits;

#[derive(Clone, Copy)]
pub(super) struct ConnectionRuntimeSignals {
    pub peers: ReadSignal<HashMap<PeerId, PeerSession>>,
    pub set_peers: WriteSignal<HashMap<PeerId, PeerSession>>,
    pub handshake_ready: ReadSignal<bool>,
    pub set_handshake_ready: WriteSignal<bool>,
    pub handshake_scope_nonce: ReadSignal<Option<u64>>,
    pub set_handshake_scope_nonce: WriteSignal<Option<u64>>,
    pub handshake_retry_nonce: ReadSignal<u64>,
    pub set_handshake_retry_nonce: WriteSignal<u64>,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
    pub pending_navigation: ReadSignal<Option<PendingNavigation>>,
    pub set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
}

pub(super) fn init_connection_runtime_signals() -> ConnectionRuntimeSignals {
    let (peers, set_peers) = signal(HashMap::<PeerId, PeerSession>::new());
    let (handshake_ready, set_handshake_ready) = signal(false);
    let (handshake_scope_nonce, set_handshake_scope_nonce) = signal(None::<u64>);
    let (handshake_retry_nonce, set_handshake_retry_nonce) = signal(0u64);
    let (pending_local_edits, set_pending_local_edits) = signal(PendingLocalEdits::new());
    let (pending_navigation, set_pending_navigation) = signal(None::<PendingNavigation>);

    ConnectionRuntimeSignals {
        peers,
        set_peers,
        handshake_ready,
        set_handshake_ready,
        handshake_scope_nonce,
        set_handshake_scope_nonce,
        handshake_retry_nonce,
        set_handshake_retry_nonce,
        pending_local_edits,
        set_pending_local_edits,
        pending_navigation,
        set_pending_navigation,
    }
}
