use leptos::task::spawn_local;
use leptos::prelude::*;
use gloo_net::websocket::{futures::WebSocket, Message};
use futures::{StreamExt, SinkExt};
use futures::channel::mpsc::{unbounded, UnboundedSender};
use deve_core::protocol::{ClientMessage, ServerMessage};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Clone)]
pub struct WsService {
    pub status: ReadSignal<ConnectionStatus>,
    pub msg: ReadSignal<Option<ServerMessage>>,
    tx: UnboundedSender<ClientMessage>,
}

impl WsService {
    pub fn new() -> Self {
        let (status, set_status) = signal(ConnectionStatus::Disconnected);
        let (msg, set_msg) = signal(None);
        let (tx, mut rx) = unbounded::<ClientMessage>();
        
        spawn_local(async move {
            set_status.set(ConnectionStatus::Connecting);
            let url = "ws://localhost:3001/ws";
            match WebSocket::open(url) {
                Ok(ws) => {
                    leptos::logging::log!("WS Connected!");
                    set_status.set(ConnectionStatus::Connected);
                    let (mut write, mut read) = ws.split();
                    
                    // Task 1: Writer (Channel -> WS)
                    spawn_local(async move {
                        while let Some(msg) = rx.next().await {
                            if let Ok(json) = serde_json::to_string(&msg) {
                                if let Err(e) = write.send(Message::Text(json)).await {
                                    leptos::logging::error!("WS Write Error: {:?}", e);
                                    break;
                                }
                            }
                        }
                    });
                    
                    // Task 2: Reader (WS -> Signal)
                    while let Some(msg) = read.next().await {
                        match msg {
                             Ok(Message::Text(txt)) => {
                                 // leptos::logging::log!("WS Recv: {}", txt);
                                 if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&txt) {
                                     set_msg.set(Some(server_msg));
                                 }
                             }
                             _ => {}
                        }
                    }
                    set_status.set(ConnectionStatus::Disconnected);
                }
                Err(e) => {
                    leptos::logging::error!("WS Error: {:?}", e);
                    set_status.set(ConnectionStatus::Disconnected);
                }
            }
        });
        
        Self { status, msg, tx }
    }

    pub fn send(&self, msg: ClientMessage) {
        if let Err(e) = self.tx.unbounded_send(msg) {
            leptos::logging::error!("Failed to enqueue message: {:?}", e);
        }
    }
}
