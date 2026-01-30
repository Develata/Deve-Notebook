// apps/cli/src/server/channel.rs
//! # 响应通道抽象 (Response Channel)
//!
//! **功能**:
//! 提供统一的消息发送接口，区分广播 (Broadcast) 和单播 (Unicast)。
//!
//! **设计原则**:
//! - Broadcast: 用于全局事件（文件变更、同步完成）
//! - Unicast: 用于单客户端响应（Error、Ack、Pong）
//!
//! **使用场景**:
//! Handler 函数接收 `ResponseChannel` 参数，根据消息类型选择合适的通道发送。

use deve_core::protocol::ServerMessage;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{broadcast, mpsc};

/// 响应通道类型
///
/// 封装广播和单播两种发送方式，提供统一的 API。
#[allow(dead_code)] // 为 Handler 模块预留的通道抽象
#[derive(Clone)]
pub enum ResponseChannel {
    /// 广播通道 - 发送给所有连接的客户端
    Broadcast(broadcast::Sender<ServerMessage>),
    /// 单播通道 - 仅发送给当前客户端
    Unicast(mpsc::Sender<ServerMessage>),
}

#[allow(dead_code)] // 为 Handler 模块预留的通道抽象
impl ResponseChannel {
    /// 发送消息
    ///
    /// 根据通道类型选择合适的发送方式
    pub fn send(&self, msg: ServerMessage) {
        match self {
            ResponseChannel::Broadcast(tx) => {
                let _ = tx.send(msg);
            }
            ResponseChannel::Unicast(tx) => {
                if let Err(e) = tx.try_send(msg) {
                    match e {
                        TrySendError::Full(_) => {
                            tracing::warn!("Unicast channel full; dropping message");
                        }
                        TrySendError::Closed(_) => {
                            tracing::debug!("Unicast channel closed; dropping message");
                        }
                    }
                }
            }
        }
    }
}

/// 双通道上下文
///
/// 同时持有广播和单播通道，供 Handler 按需选择。
#[derive(Clone)]
pub struct DualChannel {
    /// 广播通道 - 全局事件
    pub broadcast: broadcast::Sender<ServerMessage>,
    /// 单播通道 - 单客户端响应
    pub unicast: mpsc::Sender<ServerMessage>,
}

impl DualChannel {
    /// 创建双通道上下文
    pub fn new(
        broadcast: broadcast::Sender<ServerMessage>,
        unicast: mpsc::Sender<ServerMessage>,
    ) -> Self {
        Self { broadcast, unicast }
    }

    /// 广播消息 (全局事件)
    pub fn broadcast(&self, msg: ServerMessage) {
        let _ = self.broadcast.send(msg);
    }

    /// 单播消息 (单客户端响应)
    pub fn unicast(&self, msg: ServerMessage) {
        if let Err(e) = self.unicast.try_send(msg) {
            match e {
                TrySendError::Full(_) => {
                    tracing::warn!("Unicast channel full; dropping message");
                }
                TrySendError::Closed(_) => {
                    tracing::debug!("Unicast channel closed; dropping message");
                }
            }
        }
    }

    /// 发送错误响应 (自动使用单播)
    pub fn send_error(&self, message: String) {
        self.unicast(ServerMessage::Error(message));
    }
}
