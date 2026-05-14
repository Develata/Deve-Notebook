//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::api::WsService;
use crate::i18n::Locale;
use deve_core::protocol::ServerMessage;

use super::super::state::CoreSignals;
use super::message_dispatch_runtime::{
    handle_chat_chunk_message, handle_plugin_response_message, handle_search_results_message,
};

pub fn route_runtime_message(
    msg: ServerMessage,
    _ws: &WsService,
    locale: Locale,
    signals: CoreSignals,
) -> Option<ServerMessage> {
    match msg {
        ServerMessage::PluginResponse {
            req_id,
            result,
            error,
        } => {
            handle_plugin_response_message(req_id, result, error, locale, signals);
            None
        }
        ServerMessage::ChatChunk {
            req_id,
            delta,
            finish_reason,
        } => {
            handle_chat_chunk_message(req_id, delta, finish_reason, signals);
            None
        }
        ServerMessage::SearchResults {
            request_id,
            repo_id,
            branch,
            scope_nonce,
            results,
        } => {
            handle_search_results_message(
                request_id,
                repo_id,
                branch,
                scope_nonce,
                results,
                signals,
            );
            None
        }
        other => Some(other),
    }
}
