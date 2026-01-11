//! WebSocket connection manager.
//!
//! Responsibilities:
//! 1. Establish WebSocket connection
//! 2. Exponential backoff reconnection
//! 3. Read server messages and update signals
//! 4. Pass the write-half to Output Manager via link_tx

use leptos::task::spawn_local;
use leptos::prelude::*;
use gloo_net::websocket::{futures::WebSocket, Message};
use futures::StreamExt;
use futures::channel::mpsc::UnboundedSender;
use futures::stream::SplitSink;
use deve_core::protocol::ServerMessage;

use super::backoff::BackoffStrategy;
use super::ConnectionStatus;

/// Spawns the connection manager task.
pub fn spawn_connection_manager(
    set_status: WriteSignal<ConnectionStatus>,
    set_msg: WriteSignal<Option<ServerMessage>>,
    link_tx: UnboundedSender<SplitSink<WebSocket, Message>>,
) {
    spawn_local(async move {
        let url = build_ws_url();
        let mut backoff = BackoffStrategy::new();
        
        loop {
            set_status.set(ConnectionStatus::Connecting);
            leptos::logging::log!("WS: Connecting to {}...", url);
            
            match WebSocket::open(&url) {
                Ok(ws) => {
                    leptos::logging::log!("WS: Connected!");
                    set_status.set(ConnectionStatus::Connected);
                    backoff.reset();
                    
                    let (write, read) = ws.split();
                    
                    // Hand over the writer to the Output Manager
                    if let Err(e) = link_tx.unbounded_send(write) {
                        leptos::logging::error!("Failed to send sink to output loop: {:?}", e);
                    }
                    
                    // Block on reading until disconnect
                    process_incoming_messages(read, set_msg.clone()).await;
                    
                    leptos::logging::log!("WS: Connection Lost (Reader ended)");
                }
                Err(e) => {
                    leptos::logging::error!("WS Open Error: {:?}", e);
                }
            }
            
            set_status.set(ConnectionStatus::Disconnected);
            backoff.wait().await;
        }
    });
}

/// Reads messages from the WebSocket until the connection is closed.
async fn process_incoming_messages(
    mut read: futures::stream::SplitStream<WebSocket>,
    set_msg: WriteSignal<Option<ServerMessage>>,
) {
    while let Some(result) = read.next().await {
        match result {
            Ok(Message::Text(txt)) => {
                match serde_json::from_str::<ServerMessage>(&txt) {
                    Ok(server_msg) => set_msg.set(Some(server_msg)),
                    Err(e) => leptos::logging::error!("Parse Error: {:?}", e),
                }
            }
            Ok(_) => {} // Ignore binary messages
            Err(e) => {
                leptos::logging::error!("WS Read Error: {:?}", e);
                break;
            }
        }
    }
}

/// Builds the WebSocket URL based on the current hostname.
fn build_ws_url() -> String {
    let hostname = web_sys::window()
        .unwrap()
        .location()
        .hostname()
        .unwrap_or_else(|_| "localhost".to_string());
    format!("ws://{}:3001/ws", hostname)
}
