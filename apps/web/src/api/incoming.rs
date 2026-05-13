//! plan_ref:
//!   - 05_network#web-ws-runtime
//!

use self::decode::{decode_binary_message, decode_text_message};
use super::ConnectionStatus;
use super::socket::{SocketEvent, SocketMessage};
use deve_core::protocol::ServerMessage;
use leptos::prelude::*;
use std::collections::VecDeque;

mod decode;
#[cfg(test)]
mod tests;

const MAX_INCOMING_MESSAGES: usize = 256;

pub fn enqueue_server_message(
    set_msg_seq: WriteSignal<u64>,
    set_msg_queue: WriteSignal<VecDeque<(u64, u64, ServerMessage)>>,
    connection_epoch: u64,
    server_msg: ServerMessage,
) -> bool {
    let Some(next_seq) = set_msg_seq.try_update(|seq| {
        *seq = seq.saturating_add(1);
        *seq
    }) else {
        return false;
    };
    set_msg_queue
        .try_update(move |queue| {
            push_server_message(queue, next_seq, connection_epoch, server_msg);
        })
        .is_some()
}

pub fn handle_socket_event(
    event: SocketEvent,
    confirmed_connected: &mut bool,
    set_msg_seq: WriteSignal<u64>,
    set_msg_queue: WriteSignal<VecDeque<(u64, u64, ServerMessage)>>,
    set_status: WriteSignal<ConnectionStatus>,
    connection_epoch: u64,
) -> bool {
    match event {
        SocketEvent::Opened => {}
        SocketEvent::Message(SocketMessage::Bytes(bytes)) => {
            if let Some(server_msg) = decode_binary_message(&bytes) {
                return confirm_and_enqueue_server_message(
                    confirmed_connected,
                    set_status,
                    set_msg_seq,
                    set_msg_queue,
                    connection_epoch,
                    "binary",
                    server_msg,
                );
            }
        }
        SocketEvent::Message(SocketMessage::Text(txt)) => {
            if let Some(server_msg) = decode_text_message(&txt) {
                return confirm_and_enqueue_server_message(
                    confirmed_connected,
                    set_status,
                    set_msg_seq,
                    set_msg_queue,
                    connection_epoch,
                    "text",
                    server_msg,
                );
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

fn confirm_and_enqueue_server_message(
    confirmed_connected: &mut bool,
    set_status: WriteSignal<ConnectionStatus>,
    set_msg_seq: WriteSignal<u64>,
    set_msg_queue: WriteSignal<VecDeque<(u64, u64, ServerMessage)>>,
    connection_epoch: u64,
    transport: &str,
    server_msg: ServerMessage,
) -> bool {
    confirm_connection(confirmed_connected, set_status, transport)
        && enqueue_server_message(set_msg_seq, set_msg_queue, connection_epoch, server_msg)
}

fn confirm_connection(
    confirmed_connected: &mut bool,
    set_status: WriteSignal<ConnectionStatus>,
    transport: &str,
) -> bool {
    if *confirmed_connected {
        return true;
    }
    leptos::logging::log!(
        "WS: First {} message received, connection confirmed!",
        transport
    );
    if set_status.try_set(ConnectionStatus::Connected).is_some() {
        return false;
    }
    *confirmed_connected = true;
    true
}

fn push_server_message(
    queue: &mut VecDeque<(u64, u64, ServerMessage)>,
    seq: u64,
    connection_epoch: u64,
    server_msg: ServerMessage,
) {
    queue.push_back((seq, connection_epoch, server_msg));
    while queue.len() > MAX_INCOMING_MESSAGES {
        queue.pop_front();
    }
}
