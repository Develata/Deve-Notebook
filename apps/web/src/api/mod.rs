// apps\web\src\api
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
use leptos::task::spawn_local;
use gloo_net::websocket::{futures::WebSocket, Message};
use futures::channel::mpsc::{unbounded, UnboundedSender};
use futures::stream::SplitSink;
use deve_core::protocol::{ClientMessage, ServerMessage};

use self::connection::spawn_connection_manager;
use self::output::spawn_output_manager;

// ============================================================================
// Types
// ============================================================================

/// WebSocket 连接状态枚举
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

/// 与后端通信的 WebSocket 服务
#[derive(Clone)]
pub struct WsService {
    /// 当前连接状态 (响应式信号)
    pub status: ReadSignal<ConnectionStatus>,
    /// 来自服务器的最新消息 (响应式信号)
    pub msg: ReadSignal<Option<ServerMessage>>,
    /// 发送消息的通道
    tx: UnboundedSender<ClientMessage>,
}

impl WsService {
    /// 创建新的 WebSocket 服务并启动后台任务
    pub fn new() -> Self {
        let (status, set_status) = signal(ConnectionStatus::Disconnected);
        let (msg, set_msg) = signal(None);
        let (tx, rx) = unbounded::<ClientMessage>();
        
        // Channel to pass the Write-half of the WS to the Output Manager
        let (link_tx, link_rx) = unbounded::<SplitSink<WebSocket, Message>>();
        
        // Spawn the two async tasks
        spawn_connection_manager(set_status, set_msg, link_tx);
        spawn_output_manager(rx, link_rx);
        
        // Spawn Heartbeat Task (30s interval)
        let tx_clone = tx.clone();
        let status_check = status;
        spawn_local(async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(30_000).await;
                if status_check.get_untracked() == ConnectionStatus::Connected {
                    let _ = tx_clone.unbounded_send(ClientMessage::Ping);
                }
            }
        });
        
        Self { status, msg, tx }
    }
    
    /// 将消息排队发送到服务器。
    /// 如果离线，消息将被排队并在连接恢复时发送。
    pub fn send(&self, msg: ClientMessage) {
        if let Err(e) = self.tx.unbounded_send(msg) {
            leptos::logging::error!("Failed to enqueue message: {:?}", e);
        }
    }
}
