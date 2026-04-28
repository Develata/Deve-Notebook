//! plan_ref:
//!   - 05_network#web-ws-runtime
//!

use deve_core::protocol::ServerMessage;
use deve_core::protocol::frame::{decode_server_binary, decode_server_json};

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
            None
        }
    }
}

pub(super) fn decode_text_message(text: &str) -> Option<ServerMessage> {
    match decode_server_json(text) {
        Ok(server_msg) => Some(server_msg),
        Err(error) => {
            leptos::logging::error!("JSON Parse Error: {:?}", error);
            None
        }
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
