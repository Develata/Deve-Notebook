//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
use leptos::prelude::{Set, Update};

use super::super::state::CoreSignals;
use super::message_dispatch_gate::{
    accepts_chat_chunk, accepts_plugin_response, accepts_search_results,
};
use super::message_runtime::handle_chat_chunk;
use crate::i18n::{Locale, t};
use crate::runtime::domain::{ChatMessage, SearchHit};
use deve_core::models::{PeerId, RepoId};

pub fn handle_plugin_response_message(
    req_id: String,
    result: Option<serde_json::Value>,
    error: Option<deve_core::protocol::ServerError>,
    locale: Locale,
    signals: CoreSignals,
) {
    if !accepts_plugin_response(&req_id, signals) {
        return;
    }
    signals
        .set_plugin_request_ids
        .update(|ids| ids.retain(|id| id != &req_id));
    finish_chat_request_from_plugin_response(
        &req_id,
        result.as_ref(),
        error.as_ref(),
        locale,
        signals,
    );
    signals
        .set_plugin_response
        .set(Some((req_id, result, error)));
}

fn finish_chat_request_from_plugin_response(
    req_id: &str,
    result: Option<&serde_json::Value>,
    error: Option<&deve_core::protocol::ServerError>,
    locale: Locale,
    signals: CoreSignals,
) {
    let response_text = plugin_response_text(result);
    let error_text = error.map(|err| t::server_error::message(locale, err.code));
    let mut matched_chat_message = false;
    signals.set_chat_messages.update(|messages| {
        let Some(message) = messages
            .iter_mut()
            .rev()
            .find(|msg| msg.req_id.as_deref() == Some(req_id))
        else {
            return;
        };
        matched_chat_message = true;
        if let Some(text) = response_text {
            if message.content.is_empty() {
                message.append_content(text);
            }
        } else if let Some(text) = error_text {
            append_plugin_error_text(message, text);
        }
    });
    if matched_chat_message {
        signals.set_is_chat_streaming.set(false);
    }
}

fn append_plugin_error_text(message: &mut ChatMessage, detail: &str) {
    if detail.is_empty() || message.content.contains(detail) {
        return;
    }
    if !message.content.is_empty() {
        message.append_content("\n\n");
    }
    message.append_content(detail);
}

fn plugin_response_text(result: Option<&serde_json::Value>) -> Option<&str> {
    let result = result?;
    if result.get("type").and_then(|value| value.as_str()) != Some("text") {
        return None;
    }
    result.get("content").and_then(|value| value.as_str())
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
    results: Vec<SearchHit>,
    signals: CoreSignals,
) {
    if !accepts_search_results(&request_id, repo_id, branch, scope_nonce, signals) {
        return;
    }
    signals.set_search_request_id.set(None);
    signals.set_search_results.set(results);
}

#[cfg(test)]
mod tests;
