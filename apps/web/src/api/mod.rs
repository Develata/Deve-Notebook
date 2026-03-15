// apps/web/src/api/mod.rs
//! # WebSocket API 模块
//!
//! 本模块提供 `WsService` 用于与后端进行 WebSocket 通信。
//!
//! ## 功能
//! - 连接管理与自动重连 (指数退避策略)
//! - 离线消息队列支持
//! - 基于信号的响应式状态更新
//! - 心跳保活 (30秒间隔)
//!
//! ## 架构设计
//!
//! ### 所有权转移模式 (神来之笔)
//! 使用 `unbounded` channel 将 WebSocket 写入句柄从连接任务"过继"给输出任务：
//! - Connection Manager: 负责建立连接、重连、读取
//! - Output Manager: 负责队列缓冲、写入
//! - 避免了 `Arc<Mutex<Option<Sink>>>` 带来的锁竞争和死锁风险
//!
//! ### 性能提示
//! 入站消息会进入一个有序小队列。消费端必须按顺序 drain，不能依赖“最新值槽位”。

mod auth_probe;
mod backoff;
mod connection;
mod incoming;
mod output;
mod writer_id;

use deve_core::protocol::{ClientMessage, ServerMessage};
use futures::channel::mpsc::{UnboundedSender, unbounded};
use futures::stream::SplitSink;
use gloo_net::websocket::{Message, futures::WebSocket};
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::VecDeque;

use self::connection::spawn_connection_manager;
use self::output::spawn_output_manager;
use self::writer_id::derive_writer_client_id;

// ============================================================================
// Types
// ============================================================================

/// WebSocket 连接状态枚举
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnectionStatus {
    /// 已断开连接
    Disconnected,
    /// 正在连接中
    Connecting,
    /// 登录态已失效
    Unauthorized,
    /// 已成功连接
    Connected,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Disconnected => write!(f, "Disconnected"),
            ConnectionStatus::Connecting => write!(f, "Connecting..."),
            ConnectionStatus::Unauthorized => write!(f, "Unauthorized"),
            ConnectionStatus::Connected => write!(f, "Connected"),
        }
    }
}

// ============================================================================
// WsService - Public API
// ============================================================================

/// 与后端通信的 WebSocket 服务
///
/// ## 使用示例
/// ```ignore
/// let ws = WsService::new();
/// ws.send(ClientMessage::ListDocs);
/// ```
///
/// ## 响应式消费
/// ```ignore
/// let (last_seq, set_last_seq) = signal(0u64);
/// Effect::new(move |_| {
///     let _ = ws.msg_seq.get();
///     for (seq, msg) in ws.messages_since(last_seq.get_untracked()) {
///         // 处理消息...
///         set_last_seq.set(seq);
///     }
/// });
/// ```
#[allow(dead_code)] // endpoint/node_role: 为多节点架构预留
#[derive(Clone)]
pub struct WsService {
    /// 当前连接状态 (响应式信号)
    pub status: ReadSignal<ConnectionStatus>,
    set_status: WriteSignal<ConnectionStatus>,
    pub writer_ready_repo_id: ReadSignal<Option<String>>,
    set_writer_ready_repo_id: WriteSignal<Option<String>>,
    pub writer_client_id: ReadSignal<Option<u64>>,
    set_writer_client_id: WriteSignal<Option<u64>>,

    /// 当前连接的端点 (ws url)
    pub endpoint: ReadSignal<String>,

    /// 当前节点角色 (main/proxy)
    pub node_role: ReadSignal<String>,

    /// 入站消息序号（每收到一条消息严格递增）
    pub msg_seq: ReadSignal<u64>,
    msg_queue: ReadSignal<VecDeque<(u64, ServerMessage)>>,

    /// 发送消息的通道 (内部使用)
    tx: UnboundedSender<ClientMessage>,
}

impl WsService {
    /// 创建新的 WebSocket 服务并启动后台任务
    ///
    /// 自动启动:
    /// - Connection Manager (连接/重连/读取)
    /// - Output Manager (消息队列/写入)
    /// - Heartbeat Task (30秒心跳)
    pub fn new() -> Self {
        let (status, set_status) = signal(ConnectionStatus::Disconnected);
        let (writer_ready_repo_id, set_writer_ready_repo_id) = signal(None::<String>);
        let (writer_client_id, set_writer_client_id) = signal(None::<u64>);
        let (msg_seq, set_msg_seq) = signal(0u64);
        let (msg_queue, set_msg_queue) = signal(VecDeque::<(u64, ServerMessage)>::new());
        let (endpoint, set_endpoint) = signal(String::new());
        let (node_role, set_node_role) = signal(String::new());
        let (tx, rx) = unbounded::<ClientMessage>();

        // 所有权转移通道: 将 WebSocket 写入句柄从 Connection 传递给 Output
        // 这避免了使用 Arc<Mutex<Option<Sink>>> 带来的锁竞争
        let (link_tx, link_rx) = unbounded::<SplitSink<WebSocket, Message>>();

        // 启动两个异步任务
        spawn_connection_manager(
            set_status,
            set_msg_seq,
            set_msg_queue,
            set_endpoint,
            set_node_role,
            link_tx,
        );
        spawn_output_manager(rx, link_rx, status, set_status);

        // 启动心跳任务 (30秒间隔)
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

        Self {
            status,
            set_status,
            writer_ready_repo_id,
            set_writer_ready_repo_id,
            writer_client_id,
            set_writer_client_id,
            endpoint,
            node_role,
            msg_seq,
            msg_queue,
            tx,
        }
    }

    /// 将消息排队发送到服务器
    ///
    /// 如果当前离线，消息将被缓存并在连接恢复时自动发送。
    pub fn send(&self, msg: ClientMessage) {
        if let Err(e) = self.tx.unbounded_send(msg) {
            leptos::logging::error!("消息入队失败: {:?}", e);
        }
    }

    pub fn mark_unauthorized(&self) {
        self.clear_writer_ready();
        self.set_status.set(ConnectionStatus::Unauthorized);
    }

    pub fn mark_writer_ready(&self, repo_id: impl Into<String>, peer_id: &str) {
        self.set_writer_ready_repo_id.set(Some(repo_id.into()));
        self.set_writer_client_id
            .set(Some(derive_writer_client_id(peer_id)));
    }

    pub fn clear_writer_ready(&self) {
        self.set_writer_ready_repo_id.set(None);
        self.set_writer_client_id.set(None);
    }

    pub fn writer_ready_for(&self, repo_id: Option<&str>) -> bool {
        match (self.writer_ready_repo_id.get_untracked(), repo_id) {
            (Some(ready_repo_id), Some(repo_id)) => ready_repo_id == repo_id,
            _ => false,
        }
    }

    pub fn writer_client_id_for(&self, repo_id: Option<&str>) -> Option<u64> {
        match (
            self.writer_ready_repo_id.get_untracked(),
            self.writer_client_id.get_untracked(),
            repo_id,
        ) {
            (Some(ready_repo_id), Some(client_id), Some(repo_id)) if ready_repo_id == repo_id => {
                Some(client_id)
            }
            _ => None,
        }
    }

    pub fn messages_since(&self, after_seq: u64) -> Vec<(u64, ServerMessage)> {
        self.msg_queue
            .get_untracked()
            .into_iter()
            .filter(|(seq, _)| *seq > after_seq)
            .collect()
    }
}
