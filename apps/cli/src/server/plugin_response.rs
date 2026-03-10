use crate::server::channel::DualChannel;
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};

pub fn send_plugin_result(ch: &DualChannel, req_id: String, result: serde_json::Value) {
    ch.unicast(ServerMessage::PluginResponse {
        req_id,
        result: Some(result),
        error: None,
    });
}

pub fn send_plugin_error(ch: &DualChannel, req_id: &str, error: ServerError) {
    ch.unicast(ServerMessage::PluginResponse {
        req_id: req_id.to_string(),
        result: None,
        error: Some(error),
    });
}

pub fn send_plugin_invalid_message(ch: &DualChannel, req_id: &str, detail: impl Into<String>) {
    send_plugin_error(
        ch,
        req_id,
        ServerError::with_detail(ServerErrorCode::PluginInvalidMessage, detail),
    );
}

pub fn send_plugin_request_failed(ch: &DualChannel, req_id: &str, detail: impl Into<String>) {
    send_plugin_error(
        ch,
        req_id,
        ServerError::with_detail(ServerErrorCode::RequestFailed, detail),
    );
}
