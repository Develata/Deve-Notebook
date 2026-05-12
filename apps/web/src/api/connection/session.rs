//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 09_auth#unauthorized-disconnected-ui
//!

use super::super::ConnectionStatus;
use super::super::incoming::handle_socket_event;
use super::super::output::{is_write_message, send_or_requeue};
use super::super::socket::{BrowserSocket, SocketEvent};
use super::ConnectionLifecycle;
use futures::FutureExt;
use futures::StreamExt;
use futures::channel::mpsc::UnboundedReceiver;
use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;
use std::collections::VecDeque;

#[derive(Clone)]
pub(super) struct ConnectedSessionSignals {
    pub lifecycle: ConnectionLifecycle,
    pub set_status: WriteSignal<ConnectionStatus>,
    pub set_msg_seq: WriteSignal<u64>,
    pub set_msg_queue: WriteSignal<VecDeque<(u64, u64, deve_core::protocol::ServerMessage)>>,
    pub connection_epoch: u64,
}

pub(super) async fn run_connected_session(
    socket: BrowserSocket,
    mut events: UnboundedReceiver<SocketEvent>,
    rx: &mut UnboundedReceiver<deve_core::protocol::ClientMessage>,
    queue: &mut VecDeque<deve_core::protocol::ClientMessage>,
    signals: ConnectedSessionSignals,
) {
    let mut confirmed_connected = false;
    let mut announced_open = false;

    loop {
        if !signals.lifecycle.is_active() {
            return;
        }
        if socket.is_open() && !announced_open {
            leptos::logging::log!("WS: Socket opened, waiting for first message...");
            announced_open = true;
        }

        if socket.is_closed() {
            leptos::logging::warn!(
                "WS session ended because browser socket is closed: ready_state={}",
                socket.ready_state()
            );
            return;
        }

        if socket.is_open()
            && let Some(msg) = queue.pop_front()
            && !send_or_requeue(&socket, msg, queue)
        {
            return;
        }

        let inbound = events.next().fuse();
        let outbound = rx.next().fuse();
        let timer = TimeoutFuture::new(25).fuse();
        futures::pin_mut!(inbound, outbound, timer);

        futures::select! {
            result = inbound => match result {
                Some(event) => {
                    if matches!(event, SocketEvent::Opened) && !announced_open && socket.is_open() {
                        leptos::logging::log!("WS: Socket opened, waiting for first message...");
                        announced_open = true;
                    }
                    if !handle_socket_event(
                        event,
                        &mut confirmed_connected,
                        signals.set_msg_seq,
                        signals.set_msg_queue,
                        signals.set_status,
                        signals.connection_epoch,
                    ) {
                        return;
                    }
                }
                None => {
                    if socket.is_closed() {
                        return;
                    }
                }
            },
            maybe_msg = outbound => match maybe_msg {
                Some(msg) => {
                    if !confirmed_connected && is_write_message(&msg) {
                        leptos::logging::warn!("WebLightPeer: 断连时禁止写入消息 {:?}", msg);
                        continue;
                    }
                    queue.push_back(msg);
                }
                None => return,
            },
            _ = timer => {}
        }
    }
}
