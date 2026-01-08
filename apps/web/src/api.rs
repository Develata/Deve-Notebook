use leptos::task::spawn_local;
use leptos::prelude::*;
use gloo_net::websocket::{futures::WebSocket, Message};
use futures::{StreamExt, SinkExt};
use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::stream::SplitSink;
use std::collections::VecDeque;
use deve_core::protocol::{ClientMessage, ServerMessage};
use gloo_timers::future::TimeoutFuture;

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

enum OutputLoopMsg {
    Client(ClientMessage),
    NewLink(SplitSink<WebSocket, Message>),
}

impl WsService {
    pub fn new() -> Self {
        let (status, set_status) = signal(ConnectionStatus::Disconnected);
        let (msg, set_msg) = signal(None);
        let (tx, rx) = unbounded::<ClientMessage>();
        
        // Channel to pass the Write-half of the WS to the Output Loop
        let (link_tx, link_rx) = unbounded::<SplitSink<WebSocket, Message>>();
        
        // Task 1: Connection Manager (The "Eye")
        spawn_local(async move {
            let url = "ws://localhost:3001/ws";
            let mut backoff = 1000u32;
            
            loop {
                set_status.set(ConnectionStatus::Connecting);
                leptos::logging::log!("WS: Connecting...");
                
                match WebSocket::open(url) {
                    Ok(ws) => {
                        leptos::logging::log!("WS: Connected!");
                        set_status.set(ConnectionStatus::Connected);
                        backoff = 1000; // Reset backoff
                        
                        let (write, mut read) = ws.split();
                        
                        // Hand over the writer to the Output Loop
                        if let Err(e) = link_tx.unbounded_send(write) {
                            leptos::logging::error!("Failed to send sink to output loop: {:?}", e);
                        }
                        
                        // Reader Loop - Blocks until disconnect
                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(txt)) => {
                                    match serde_json::from_str::<ServerMessage>(&txt) {
                                        Ok(server_msg) => set_msg.set(Some(server_msg)),
                                        Err(e) => leptos::logging::error!("Parse Error: {:?}", e),
                                    }
                                }
                                Ok(_) => {}, // Ignore bytes for now
                                Err(e) => {
                                    leptos::logging::error!("WS Read Error: {:?}", e);
                                    break;
                                }
                            }
                        }
                        
                        leptos::logging::log!("WS: Connection Lost (Reader ended)");
                    }
                    Err(e) => {
                        leptos::logging::error!("WS Open Error: {:?}", e);
                    }
                }
                
                set_status.set(ConnectionStatus::Disconnected);
                
                // Exponential Backoff with Cap (max 10s)
                leptos::logging::log!("WS: Reconnecting in {}ms...", backoff);
                TimeoutFuture::new(backoff).await;
                backoff = (backoff * 2).min(10000);
            }
        });
        
        // Task 2: Output Manager (The "Hand")
        // Owns the current Writer and the Offline Queue
        spawn_local(async move {
            let mut current_sink: Option<SplitSink<WebSocket, Message>> = None;
            let mut queue: VecDeque<ClientMessage> = VecDeque::new();
            
            // Merge streams: Client Messages + New Connection Links
            let mut start_stream = futures::stream::select(
                rx.map(OutputLoopMsg::Client),
                link_rx.map(OutputLoopMsg::NewLink)
            );
            
            while let Some(event) = start_stream.next().await {
                match event {
                    OutputLoopMsg::NewLink(sink) => {
                        leptos::logging::log!("OutputLoop: New Connection Link received. Flushing {} items.", queue.len() + 1);
                        current_sink = Some(sink);
                        
                        // 1. Inject Re-Sync Message (ListDocs) at front
                        // This ensures we refresh the state before applying queued edits
                        // Note: If we had a "Version" check, we might want Snapshot instead.
                        // For now, ListDocs is safe.
                        // Actually, pushing to FRONT means it processes LAST if we iterate forward?
                        // No, push_front + pop_front = Stack? No.
                        // queue is FIFO. pop_front gets simplest.
                        // We want ListDocs to be FIRST. So push_front.
                        queue.push_front(ClientMessage::ListDocs);
                        
                        // 2. Flush Queue
                        // We must process ALL items currently in queue.
                        // If any fail, we stop flushing and wait for next Link.
                        let count = queue.len();
                        for _ in 0..count {
                            if let Some(msg) = queue.pop_front() {
                                let json = match serde_json::to_string(&msg) {
                                    Ok(j) => j,
                                    Err(_) => continue, // Drop invalid json
                                };
                                
                                let writer = current_sink.as_mut().unwrap(); // Safe because we just set Some
                                match writer.send(Message::Text(json)).await {
                                    Ok(_) => {},
                                    Err(e) => {
                                        leptos::logging::error!("WS Flush Error: {:?}. Connection likely died.", e);
                                        // Put it back!
                                        queue.push_front(msg); // Return to front
                                        // Writer is dead
                                        current_sink = None;
                                        break; // Stop flushing, wait for new link
                                    }
                                }
                            }
                        }
                    },
                    OutputLoopMsg::Client(msg) => {
                         if let Some(writer) = current_sink.as_mut() {
                             // Try sending
                             let json = serde_json::to_string(&msg).unwrap_or_default();
                             if let Err(e) = writer.send(Message::Text(json)).await {
                                  leptos::logging::warn!("WS Send Error: {:?}. Queuing...", e);
                                  queue.push_back(msg);
                                  current_sink = None; // Mark as dead
                             }
                         } else {
                             // Offline, just queue
                             queue.push_back(msg);
                         }
                    }
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
