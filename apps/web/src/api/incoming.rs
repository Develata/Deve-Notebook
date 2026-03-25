use super::ConnectionStatus;
use super::socket::{SocketEvent, SocketMessage};
use deve_core::protocol::ServerMessage;
use leptos::prelude::*;
use std::collections::VecDeque;

const MAX_INCOMING_MESSAGES: usize = 256;
const FRAME_PREVIEW_BYTES: usize = 16;

pub fn enqueue_server_message(
    set_msg_seq: WriteSignal<u64>,
    set_msg_queue: WriteSignal<VecDeque<(u64, ServerMessage)>>,
    server_msg: ServerMessage,
) {
    let mut next_seq = 0u64;
    set_msg_seq.update(|seq| {
        *seq = seq.saturating_add(1);
        next_seq = *seq;
    });
    set_msg_queue.update(move |queue| {
        push_server_message(queue, next_seq, server_msg);
    });
}

pub fn handle_socket_event(
    event: SocketEvent,
    confirmed_connected: &mut bool,
    set_msg_seq: WriteSignal<u64>,
    set_msg_queue: WriteSignal<VecDeque<(u64, ServerMessage)>>,
    set_status: WriteSignal<ConnectionStatus>,
) -> bool {
    match event {
        SocketEvent::Opened => {}
        SocketEvent::Message(SocketMessage::Bytes(bytes)) => {
            if let Some(server_msg) = decode_binary_message(&bytes) {
                confirm_connection(confirmed_connected, set_status, "binary");
                enqueue_server_message(set_msg_seq, set_msg_queue, server_msg);
            }
        }
        SocketEvent::Message(SocketMessage::Text(txt)) => {
            match serde_json::from_str::<ServerMessage>(&txt) {
                Ok(server_msg) => {
                    confirm_connection(confirmed_connected, set_status, "text");
                    enqueue_server_message(set_msg_seq, set_msg_queue, server_msg);
                }
                Err(e) => leptos::logging::error!("JSON Parse Error: {:?}", e),
            }
        }
        SocketEvent::Error => {
            leptos::logging::error!("WS Read Error: browser error event");
            return false;
        }
        SocketEvent::Closed(info) => {
            leptos::logging::warn!(
                "WS Closed: code={}, clean={}, reason={}",
                info.code,
                info.was_clean,
                info.reason
            );
            return false;
        }
    }
    true
}

fn confirm_connection(
    confirmed_connected: &mut bool,
    set_status: WriteSignal<ConnectionStatus>,
    transport: &str,
) {
    if *confirmed_connected {
        return;
    }
    leptos::logging::log!(
        "WS: First {} message received, connection confirmed!",
        transport
    );
    set_status.set(ConnectionStatus::Connected);
    *confirmed_connected = true;
}

fn decode_binary_message(bytes: &[u8]) -> Option<ServerMessage> {
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

fn push_server_message(
    queue: &mut VecDeque<(u64, ServerMessage)>,
    seq: u64,
    server_msg: ServerMessage,
) {
    queue.push_back((seq, server_msg));
    while queue.len() > MAX_INCOMING_MESSAGES {
        queue.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::decode_binary_message;
    use super::push_server_message;
    use deve_core::models::Op;
    use deve_core::protocol::ConfirmedOp;
    use deve_core::protocol::ServerMessage;
    use std::collections::VecDeque;

    #[test]
    fn incoming_queue_keeps_latest_messages_in_order() {
        let mut queue = VecDeque::new();
        for seq in 1..=300 {
            push_server_message(&mut queue, seq, ServerMessage::Pong);
        }

        assert_eq!(queue.len(), 256);
        assert_eq!(queue.front().map(|(seq, _)| *seq), Some(45));
        assert_eq!(queue.back().map(|(seq, _)| *seq), Some(300));
    }

    #[test]
    fn binary_json_fallback_still_decodes_server_message() {
        let bytes = br#""Pong""#;
        assert!(matches!(
            decode_binary_message(bytes),
            Some(ServerMessage::Pong)
        ));
    }

    #[test]
    fn binary_bincode_still_decodes_server_message() {
        let bytes = bincode::serialize(&ServerMessage::Pong).unwrap();
        assert!(matches!(
            decode_binary_message(&bytes),
            Some(ServerMessage::Pong)
        ));
    }

    #[test]
    fn binary_bincode_decodes_history_with_none_origin() {
        let bytes = bincode::serialize(&ServerMessage::History {
            repo_id: uuid::Uuid::nil(),
            branch: None,
            scope_nonce: Some(1),
            doc_id: deve_core::models::DocId::from_u128(7),
            request_id: 9,
            ops: vec![ConfirmedOp::new(
                3,
                Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                None,
            )],
        })
        .unwrap();
        assert!(matches!(
            decode_binary_message(&bytes),
            Some(ServerMessage::History { .. })
        ));
    }
}
