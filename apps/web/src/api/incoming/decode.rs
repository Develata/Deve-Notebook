//! plan_ref:
//!   - 05_network#web-ws-runtime
//!

use deve_core::protocol::frame::{
    MISSING_WS_FRAME_MAGIC, ProtocolFrameError, decode_server_binary, decode_server_json,
};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};

const FRAME_PREVIEW_BYTES: usize = 16;

pub(super) fn decode_binary_message(bytes: &[u8]) -> Option<ServerMessage> {
    match decode_server_binary(bytes) {
        Ok(server_msg) => Some(server_msg),
        Err(frame_error) => {
            leptos::logging::warn!(
                "Ignoring undecodable WS binary frame: len={}, head={}, err={:?}",
                bytes.len(),
                preview_bytes(bytes),
                frame_error
            );
            protocol_error_message(frame_error)
        }
    }
}

pub(super) fn decode_text_message(text: &str) -> Option<ServerMessage> {
    match decode_server_json(text) {
        Ok(server_msg) => Some(server_msg),
        Err(error) => {
            leptos::logging::error!("JSON Parse Error: {:?}", error);
            protocol_error_message(error)
        }
    }
}

fn protocol_error_message(error: ProtocolFrameError) -> Option<ServerMessage> {
    match error {
        ProtocolFrameError::UnsupportedVersion { .. } => Some(ServerMessage::ProtocolError {
            error: ServerError::with_detail(
                ServerErrorCode::SyncVersionMismatch,
                error.to_string(),
            ),
            switch_nonce: None,
            scope_nonce: None,
        }),
        ProtocolFrameError::Decode(detail) if detail == MISSING_WS_FRAME_MAGIC => None,
        ProtocolFrameError::Decode(_) => Some(ServerMessage::ProtocolError {
            error: ServerError::with_detail(
                ServerErrorCode::SyncInvalidPayload,
                "Invalid WS server frame",
            ),
            switch_nonce: None,
            scope_nonce: None,
        }),
    }
}

fn preview_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(FRAME_PREVIEW_BYTES)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
