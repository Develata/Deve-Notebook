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

use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
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
                send_unicast(tx, msg);
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
        send_unicast(&self.unicast, msg);
    }

    pub fn send_protocol_error(&self, error: ServerError) {
        self.send_protocol_error_with_switch_nonce(error, None);
    }

    pub fn send_protocol_error_with_switch_nonce(
        &self,
        error: ServerError,
        switch_nonce: Option<u64>,
    ) {
        self.unicast(ServerMessage::ProtocolError {
            error,
            switch_nonce,
        });
    }

    pub fn send_sync_repo_unbound(&self) {
        self.send_protocol_error(ServerError::new(ServerErrorCode::SyncRepoUnbound));
    }

    pub fn send_sync_peer_unauthenticated(&self) {
        self.send_protocol_error(ServerError::new(ServerErrorCode::SyncPeerUnauthenticated));
    }
}

fn send_unicast(tx: &mpsc::Sender<ServerMessage>, msg: ServerMessage) {
    let must_deliver = must_deliver_unicast(&msg);
    if let Err(error) = tx.try_send(msg) {
        match error {
            TrySendError::Full(msg) if must_deliver => schedule_must_deliver(tx.clone(), msg),
            TrySendError::Full(_) => {
                tracing::warn!("Unicast channel full; dropping message");
            }
            TrySendError::Closed(_) => {
                tracing::debug!("Unicast channel closed; dropping message");
            }
        }
    }
}

fn must_deliver_unicast(msg: &ServerMessage) -> bool {
    matches!(
        msg,
        ServerMessage::ProtocolError { .. } | ServerMessage::EditRejected { .. }
    )
}

fn schedule_must_deliver(tx: mpsc::Sender<ServerMessage>, msg: ServerMessage) {
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
            let _ = tx.send(msg).await;
        });
    } else {
        tracing::warn!("Unicast channel full outside runtime; dropping critical message");
    }
}

#[cfg(test)]
#[path = "channel_test.rs"]
mod tests;
