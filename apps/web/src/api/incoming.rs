use super::ConnectionStatus;
use deve_core::protocol::ServerMessage;
use futures::StreamExt;
use gloo_net::websocket::{Message, futures::WebSocket};
use leptos::prelude::*;
use std::collections::VecDeque;

const MAX_INCOMING_MESSAGES: usize = 256;

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

pub async fn process_incoming_messages(
    mut read: futures::stream::SplitStream<WebSocket>,
    set_msg_seq: WriteSignal<u64>,
    set_msg_queue: WriteSignal<VecDeque<(u64, ServerMessage)>>,
    set_status: WriteSignal<ConnectionStatus>,
) {
    let mut confirmed_connected = false;

    while let Some(result) = read.next().await {
        match result {
            Ok(Message::Bytes(bytes)) => {
                if !confirmed_connected {
                    leptos::logging::log!(
                        "WS: First binary message received, connection confirmed!"
                    );
                    set_status.set(ConnectionStatus::Connected);
                    confirmed_connected = true;
                }

                match bincode::deserialize::<ServerMessage>(&bytes) {
                    Ok(server_msg) => {
                        enqueue_server_message(set_msg_seq, set_msg_queue, server_msg);
                    }
                    Err(e) => leptos::logging::error!("Bincode Parse Error: {:?}", e),
                }
            }
            Ok(Message::Text(txt)) => {
                if !confirmed_connected {
                    leptos::logging::log!("WS: First text message received, connection confirmed!");
                    set_status.set(ConnectionStatus::Connected);
                    confirmed_connected = true;
                }

                match serde_json::from_str::<ServerMessage>(&txt) {
                    Ok(server_msg) => {
                        enqueue_server_message(set_msg_seq, set_msg_queue, server_msg);
                    }
                    Err(e) => leptos::logging::error!("JSON Parse Error: {:?}", e),
                }
            }
            Err(e) => {
                leptos::logging::error!("WS Read Error: {:?}", e);
                break;
            }
        }
    }
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
    use super::push_server_message;
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
}
