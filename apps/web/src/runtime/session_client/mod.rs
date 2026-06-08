//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! Browser session client runtime.
//!
//! This is a Flow Coordination adapter for transport/session readiness. It
//! does not store business truth or perform authority writes.

use crate::api::{ConnectionStatus, WsService};
use leptos::prelude::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct SessionClient {
    pub ws: WsService,
    pub connection_status: ReadSignal<ConnectionStatus>,
    pub status_text: Signal<String>,
    pub sync_banner: Signal<Option<String>>,
    pub set_sync_banner: WriteSignal<Option<String>>,
    pub handshake_ready: ReadSignal<bool>,
    pub handshake_scope_nonce: ReadSignal<Option<u64>>,
    pub on_retry_peer_registration: Callback<()>,
}
