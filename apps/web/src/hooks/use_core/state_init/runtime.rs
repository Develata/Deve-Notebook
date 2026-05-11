//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 15_release#runtime-observability
//!
use deve_core::models::PeerId;
use leptos::prelude::*;
use std::collections::HashMap;

use super::super::contexts::SystemMetricsData;
use super::super::navigation::PendingNavigation;
use super::super::pending::PendingLocalEdits;
use super::super::state::PluginResponse;
use super::super::types::{ChatMessage, PeerSession};
mod ai;
mod connection;
mod sync;
use self::ai::init_ai_runtime_signals;
use self::connection::init_connection_runtime_signals;
use self::sync::init_sync_runtime_signals;

#[derive(Clone, Copy)]
pub(super) struct RuntimeSignals {
    pub peers: ReadSignal<HashMap<PeerId, PeerSession>>,
    pub set_peers: WriteSignal<HashMap<PeerId, PeerSession>>,
    pub handshake_ready: ReadSignal<bool>,
    pub set_handshake_ready: WriteSignal<bool>,
    pub handshake_scope_nonce: ReadSignal<Option<u64>>,
    pub set_handshake_scope_nonce: WriteSignal<Option<u64>>,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
    pub pending_navigation: ReadSignal<Option<PendingNavigation>>,
    pub set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    pub plugin_response: ReadSignal<PluginResponse>,
    pub set_plugin_response: WriteSignal<PluginResponse>,
    pub plugin_request_ids: ReadSignal<Vec<String>>,
    pub set_plugin_request_ids: WriteSignal<Vec<String>>,
    pub chat_messages: ReadSignal<Vec<ChatMessage>>,
    pub set_chat_messages: WriteSignal<Vec<ChatMessage>>,
    pub is_chat_streaming: ReadSignal<bool>,
    pub set_is_chat_streaming: WriteSignal<bool>,
    pub ai_mode: ReadSignal<String>,
    pub set_ai_mode: WriteSignal<String>,
    pub search_request_id: ReadSignal<Option<String>>,
    pub set_search_request_id: WriteSignal<Option<String>>,
    pub search_results: ReadSignal<Vec<(String, String, f32)>>,
    pub set_search_results: WriteSignal<Vec<(String, String, f32)>>,
    pub load_state: ReadSignal<String>,
    pub set_load_state: WriteSignal<String>,
    pub load_progress: ReadSignal<(usize, usize)>,
    pub set_load_progress: WriteSignal<(usize, usize)>,
    pub load_eta_ms: ReadSignal<u64>,
    pub set_load_eta_ms: WriteSignal<u64>,
    pub sync_mode: ReadSignal<String>,
    pub set_sync_mode: WriteSignal<String>,
    pub sync_mode_request_id: ReadSignal<Option<String>>,
    pub set_sync_mode_request_id: WriteSignal<Option<String>>,
    pub pending_ops_count: ReadSignal<u32>,
    pub set_pending_ops_count: WriteSignal<u32>,
    pub pending_ops_previews: ReadSignal<Vec<(String, String, String)>>,
    pub set_pending_ops_previews: WriteSignal<Vec<(String, String, String)>>,
    pub pending_ops_request_id: ReadSignal<Option<String>>,
    pub set_pending_ops_request_id: WriteSignal<Option<String>>,
    pub system_metrics: ReadSignal<Option<SystemMetricsData>>,
    pub set_system_metrics: WriteSignal<Option<SystemMetricsData>>,
    pub system_metrics_live: ReadSignal<bool>,
    pub set_system_metrics_live: WriteSignal<bool>,
    pub set_explicit_home: WriteSignal<bool>,
}

pub(super) fn init_runtime_signals() -> RuntimeSignals {
    let connection = init_connection_runtime_signals();
    let ai = init_ai_runtime_signals();
    let sync = init_sync_runtime_signals();

    RuntimeSignals {
        peers: connection.peers,
        set_peers: connection.set_peers,
        handshake_ready: connection.handshake_ready,
        set_handshake_ready: connection.set_handshake_ready,
        handshake_scope_nonce: connection.handshake_scope_nonce,
        set_handshake_scope_nonce: connection.set_handshake_scope_nonce,
        pending_local_edits: connection.pending_local_edits,
        set_pending_local_edits: connection.set_pending_local_edits,
        pending_navigation: connection.pending_navigation,
        set_pending_navigation: connection.set_pending_navigation,
        plugin_response: ai.plugin_response,
        set_plugin_response: ai.set_plugin_response,
        plugin_request_ids: ai.plugin_request_ids,
        set_plugin_request_ids: ai.set_plugin_request_ids,
        chat_messages: ai.chat_messages,
        set_chat_messages: ai.set_chat_messages,
        is_chat_streaming: ai.is_chat_streaming,
        set_is_chat_streaming: ai.set_is_chat_streaming,
        ai_mode: ai.ai_mode,
        set_ai_mode: ai.set_ai_mode,
        search_request_id: ai.search_request_id,
        set_search_request_id: ai.set_search_request_id,
        search_results: ai.search_results,
        set_search_results: ai.set_search_results,
        load_state: sync.load_state,
        set_load_state: sync.set_load_state,
        load_progress: sync.load_progress,
        set_load_progress: sync.set_load_progress,
        load_eta_ms: sync.load_eta_ms,
        set_load_eta_ms: sync.set_load_eta_ms,
        sync_mode: sync.sync_mode,
        set_sync_mode: sync.set_sync_mode,
        sync_mode_request_id: sync.sync_mode_request_id,
        set_sync_mode_request_id: sync.set_sync_mode_request_id,
        pending_ops_count: sync.pending_ops_count,
        set_pending_ops_count: sync.set_pending_ops_count,
        pending_ops_previews: sync.pending_ops_previews,
        set_pending_ops_previews: sync.set_pending_ops_previews,
        pending_ops_request_id: sync.pending_ops_request_id,
        set_pending_ops_request_id: sync.set_pending_ops_request_id,
        system_metrics: sync.system_metrics,
        set_system_metrics: sync.set_system_metrics,
        system_metrics_live: sync.system_metrics_live,
        set_system_metrics_live: sync.set_system_metrics_live,
        set_explicit_home: sync.set_explicit_home,
    }
}
