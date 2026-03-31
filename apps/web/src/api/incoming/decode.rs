use deve_core::protocol::ServerMessage;

const FRAME_PREVIEW_BYTES: usize = 16;

pub(super) fn decode_binary_message(bytes: &[u8]) -> Option<ServerMessage> {
    match bincode::deserialize::<ServerMessage>(bytes) {
        Ok(server_msg) => Some(server_msg),
        Err(bincode_error) => {
            if let Ok(text) = std::str::from_utf8(bytes)
                && let Ok(server_msg) = serde_json::from_str::<ServerMessage>(text)
            {
                leptos::logging::warn!(
                    "WS binary frame fell back to JSON decode: len={}",
                    bytes.len()
                );
                return Some(server_msg);
            }
            leptos::logging::warn!(
                "Ignoring undecodable WS binary frame: len={}, head={}, err={:?}",
                bytes.len(),
                preview_bytes(bytes),
                bincode_error
            );
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
