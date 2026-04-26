use leptos::prelude::{Set, Update};

use super::super::state::CoreSignals;
use super::message_dispatch_gate::{
    accepts_chat_chunk, accepts_plugin_response, accepts_search_results,
};
use super::message_runtime::handle_chat_chunk;
use deve_core::models::{PeerId, RepoId};

pub fn handle_plugin_response_message(
    req_id: String,
    result: Option<serde_json::Value>,
    error: Option<deve_core::protocol::ServerError>,
    signals: CoreSignals,
) {
    if !accepts_plugin_response(&req_id, signals) {
        return;
    }
    signals
        .set_plugin_request_ids
        .update(|ids| ids.retain(|id| id != &req_id));
    signals
        .set_plugin_response
        .set(Some((req_id, result, error)));
}

pub fn handle_chat_chunk_message(
    req_id: String,
    delta: Option<String>,
    finish_reason: Option<String>,
    signals: CoreSignals,
) {
    if !accepts_chat_chunk(&req_id, signals) {
        return;
    }
    handle_chat_chunk(req_id, delta, finish_reason, signals);
}

pub fn handle_search_results_message(
    request_id: String,
    repo_id: Option<RepoId>,
    branch: Option<PeerId>,
    scope_nonce: Option<u64>,
    results: Vec<(String, String, f32)>,
    signals: CoreSignals,
) {
    if !accepts_search_results(&request_id, repo_id, branch, scope_nonce, signals) {
        return;
    }
    signals.set_search_results.set(results);
}
