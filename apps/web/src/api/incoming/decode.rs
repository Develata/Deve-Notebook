//! plan_ref:
//!   - 07_network#web-ws-runtime
//!

use deve_core::protocol::ServerMessage;
use deve_core::protocol::frame::{ProtocolFrameError, decode_server_binary};

const FRAME_PREVIEW_BYTES: usize = 16;

pub(super) fn decode_binary_message(bytes: &[u8]) -> Result<ServerMessage, ProtocolFrameError> {
    decode_server_binary(bytes)
}

pub(super) fn preview_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take(FRAME_PREVIEW_BYTES)
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}
