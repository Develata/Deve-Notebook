//! # WebSocket API 模块
//!
//! 本模块提供 `WsService` 用于与后端进行 WebSocket 通信。
//!
//! ## 功能
//! - 连接管理与自动重连
//! - 离线消息队列支持
//! - 基于信号的响应式状态更新

mod backoff;
mod connection;
mod output;

use leptos::prelude::*;
use gloo_net::websocket::{futures::WebSocket, Message};
use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::stream::SplitSink;
use deve_core::protocol::{ClientMessage, ServerMessage};

use self::connection::spawn_connection_manager;
use self::output::spawn_output_manager;

// ============================================================================
// Types
// ============================================================================

/// Connection status for the WebSocket.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting..."),
            ConnectionStatus::Connected => write!(f, "Connected"),
        }
    }
}

// ============================================================================
// WsService - Public API
// ============================================================================

/// WebSocket service for communicating with the backend.
#[derive(Clone)]
pub struct WsService {
    /// Current connection status (reactive signal).
    pub status: ReadSignal<ConnectionStatus>,
    /// Latest message from the server (reactive signal).
    pub msg: ReadSignal<Option<ServerMessage>>,
    /// Sender for outgoing messages.
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
