use leptos::task::spawn_local;
use leptos::prelude::*;
use gloo_net::websocket::{futures::WebSocket, Message};
use futures::{StreamExt, SinkExt};
use futures::channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use futures::stream::SplitSink;
use std::collections::VecDeque;
use deve_core::protocol::{ClientMessage, ServerMessage};
use gloo_timers::future::TimeoutFuture;

// ============================================================================
// Types
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

/// Internal message type for the Output Manager loop
enum OutputEvent {
    /// A message from the application to send to the server
    Client(ClientMessage),
    /// A new WebSocket writer from a successful connection
    NewLink(SplitSink<WebSocket, Message>),
}

// ============================================================================
// WsService - Public API
// ============================================================================

#[derive(Clone)]
pub struct WsService {
    pub status: ReadSignal<ConnectionStatus>,
    pub msg: ReadSignal<Option<ServerMessage>>,
    tx: UnboundedSender<ClientMessage>,
}

impl WsService {
    /// Creates a new WebSocket service and starts background tasks.
    pub fn new() -> Self {
        let (status, set_status) = signal(ConnectionStatus::Disconnected);
        let (msg, set_msg) = signal(None);
        let (tx, rx) = unbounded::<ClientMessage>();
        
        // Channel to pass the Write-half of the WS to the Output Manager
        let (link_tx, link_rx) = unbounded::<SplitSink<WebSocket, Message>>();
        
        // Spawn the two async tasks
        spawn_connection_manager(set_status, set_msg, link_tx);
        spawn_output_manager(rx, link_rx);
        
        Self { status, msg, tx }
    }
    
    /// Enqueues a message to be sent to the server.
    /// If offline, the message will be queued and sent when connection is restored.
    pub fn send(&self, msg: ClientMessage) {
        if let Err(e) = self.tx.unbounded_send(msg) {
            leptos::logging::error!("Failed to enqueue message: {:?}", e);
        }
    }
}

// ============================================================================
// Connection Manager (读取端)
// ============================================================================
// 职责：
// 1. 建立 WebSocket 连接
// 2. 指数退避重连
// 3. 读取服务器消息并更新信号
// 4. 将写入端通过 link_tx 传递给 Output Manager

fn spawn_connection_manager(
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

// ============================================================================
// Output Manager (写入端)
// ============================================================================
// 职责：
// 1. 接收应用层发送的消息
// 2. 维护离线队列
// 3. 当新连接建立时，刷新队列
// 4. 发送消息到服务器

fn spawn_output_manager(
    rx: UnboundedReceiver<ClientMessage>,
    link_rx: UnboundedReceiver<SplitSink<WebSocket, Message>>,
) {
    spawn_local(async move {
        let mut current_sink: Option<SplitSink<WebSocket, Message>> = None;
        let mut queue: VecDeque<ClientMessage> = VecDeque::new();
        
        // Merge streams: Client Messages + New Connection Links
        let mut events = futures::stream::select(
            rx.map(OutputEvent::Client),
            link_rx.map(OutputEvent::NewLink)
        );
        
        while let Some(event) = events.next().await {
            match event {
                OutputEvent::NewLink(sink) => {
                    handle_new_link(sink, &mut current_sink, &mut queue).await;
                },
                OutputEvent::Client(msg) => {
                    handle_client_message(msg, &mut current_sink, &mut queue).await;
                }
            }
        }
    });
}

/// Handles a new WebSocket connection link.
async fn handle_new_link(
    sink: SplitSink<WebSocket, Message>,
    current_sink: &mut Option<SplitSink<WebSocket, Message>>,
    queue: &mut VecDeque<ClientMessage>,
) {
    leptos::logging::log!("OutputLoop: New Connection Link received. Flushing {} items.", queue.len() + 1);
    *current_sink = Some(sink);
    
    // Inject ListDocs at front to refresh state
    queue.push_front(ClientMessage::ListDocs);
    
    // Flush queue
    flush_queue(current_sink, queue).await;
}

/// Handles a client message to be sent.
async fn handle_client_message(
    msg: ClientMessage,
    current_sink: &mut Option<SplitSink<WebSocket, Message>>,
    queue: &mut VecDeque<ClientMessage>,
) {
    if let Some(writer) = current_sink.as_mut() {
        let json = match serde_json::to_string(&msg) {
            Ok(j) => j,
            Err(_) => return,
        };
        
        if let Err(e) = writer.send(Message::Text(json)).await {
            leptos::logging::warn!("WS Send Error: {:?}. Queuing...", e);
            queue.push_back(msg);
            *current_sink = None; // Mark as dead
        }
    } else {
        // Offline, just queue
        queue.push_back(msg);
    }
}

/// Flushes all messages in the queue to the current connection.
async fn flush_queue(
    current_sink: &mut Option<SplitSink<WebSocket, Message>>,
    queue: &mut VecDeque<ClientMessage>,
) {
    let count = queue.len();
    for _ in 0..count {
        if let Some(msg) = queue.pop_front() {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(_) => continue,
            };
            
            let writer = match current_sink.as_mut() {
                Some(w) => w,
                None => {
                    queue.push_front(msg);
                    break;
                }
            };
            
            if let Err(e) = writer.send(Message::Text(json)).await {
                leptos::logging::error!("WS Flush Error: {:?}. Connection likely died.", e);
                queue.push_front(msg);
                *current_sink = None;
                break;
            }
        }
    }
}

// ============================================================================
// Backoff Strategy
// ============================================================================

struct BackoffStrategy {
    current_ms: u32,
}

impl BackoffStrategy {
    const INITIAL_MS: u32 = 1000;
    const MAX_MS: u32 = 10000;
    
    fn new() -> Self {
        Self { current_ms: Self::INITIAL_MS }
    }
    
    fn reset(&mut self) {
        self.current_ms = Self::INITIAL_MS;
    }
    
    async fn wait(&mut self) {
        leptos::logging::log!("WS: Reconnecting in {}ms...", self.current_ms);
        TimeoutFuture::new(self.current_ms).await;
        self.current_ms = (self.current_ms * 2).min(Self::MAX_MS);
    }
}
