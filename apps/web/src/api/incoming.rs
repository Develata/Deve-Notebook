//! plan_ref:
//!   - 07_network#web-ws-runtime
//!

use self::decode::{decode_binary_message, preview_bytes};
use super::ConnectionStatus;
use super::socket::{SocketEvent, SocketMessage};
use deve_core::protocol::ServerMessage;
use leptos::prelude::*;
use std::collections::VecDeque;

mod decode;
#[cfg(test)]
mod tests;

const MAX_INCOMING_MESSAGES: usize = 256;

pub(crate) type IncomingMessage = (u64, u64, ServerMessage);

/// A loss-aware view over the bounded incoming ring.
///
/// Once a consumer cursor falls behind the oldest retained message, returning
/// the retained suffix would let the UI observe an authority projection with a
/// missing prefix. Consumers must reconnect instead.
#[derive(Debug)]
pub(crate) enum IncomingBatch {
    Messages(Vec<IncomingMessage>),
    Gap { latest_seq: u64 },
}

pub(crate) fn messages_since(queue: &VecDeque<IncomingMessage>, after_seq: u64) -> IncomingBatch {
    let Some((oldest_seq, _, _)) = queue.front() else {
        return IncomingBatch::Messages(Vec::new());
    };
    let latest_seq = queue.back().map_or(*oldest_seq, |(seq, _, _)| *seq);
    if after_seq.saturating_add(1) < *oldest_seq {
        return IncomingBatch::Gap { latest_seq };
    }
    IncomingBatch::Messages(
        queue
            .iter()
            .filter(|(seq, _, _)| *seq > after_seq)
            .cloned()
            .collect(),
    )
}

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
        SocketEvent::Message(SocketMessage::Bytes(bytes)) => match decode_binary_message(&bytes) {
            Ok(server_msg) => {
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
            Err(error) => {
                leptos::logging::error!(
                    "Fatal WS binary frame: len={}, head={}, error={}",
                    bytes.len(),
                    preview_bytes(&bytes),
                    error
                );
                return false;
            }
        },
        SocketEvent::Message(SocketMessage::Text(txt)) => {
            leptos::logging::error!(
                "Fatal WS text frame in binary-only runtime: len={}",
                txt.len()
            );
            return false;
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
