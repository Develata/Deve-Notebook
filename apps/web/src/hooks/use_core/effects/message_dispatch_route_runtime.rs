use crate::api::WsService;
use deve_core::protocol::ServerMessage;

use super::super::state::CoreSignals;
use super::message_dispatch_runtime::{
    handle_chat_chunk_message, handle_plugin_response_message, handle_search_results_message,
};

pub fn route_runtime_message(
    msg: ServerMessage,
    _ws: &WsService,
    signals: CoreSignals,
) -> Result<(), ServerMessage> {
    match msg {
        ServerMessage::PluginResponse {
            req_id,
            result,
            error,
        } => {
            handle_plugin_response_message(req_id, result, error, signals);
            Ok(())
        }
        ServerMessage::ChatChunk {
            req_id,
            delta,
            finish_reason,
        } => {
            handle_chat_chunk_message(req_id, delta, finish_reason, signals);
            Ok(())
        }
        ServerMessage::SearchResults {
            request_id,
            scope_nonce,
            results,
        } => {
            handle_search_results_message(request_id, scope_nonce, results, signals);
            Ok(())
        }
        other => Err(other),
    }
}
