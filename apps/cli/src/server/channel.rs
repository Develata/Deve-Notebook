// apps/cli/src/server/channel.rs
//! # 响应通道抽象 (Response Channel)
//!
//! **功能**:
//! 提供统一的双通道发送接口，区分广播 (Broadcast) 和单播 (Unicast)。

#[path = "channel_delivery.rs"]
mod delivery;

use delivery::send_unicast;
pub(crate) use delivery::try_send_with_delivery_class;
use deve_core::protocol::{ServerError, ServerMessage};
use tokio::sync::{broadcast, mpsc};

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
        self.send_protocol_error_with_scope_and_switch_nonce(error, None, None);
    }

    pub fn send_protocol_error_with_scope_nonce(
        &self,
        error: ServerError,
        scope_nonce: Option<u64>,
    ) {
        self.send_protocol_error_with_scope_and_switch_nonce(error, scope_nonce, None);
    }

    pub fn send_protocol_error_with_switch_nonce(
        &self,
        error: ServerError,
        switch_nonce: Option<u64>,
    ) {
        self.send_protocol_error_with_scope_and_switch_nonce(error, None, switch_nonce);
    }

    pub fn send_protocol_error_with_scope_and_switch_nonce(
        &self,
        error: ServerError,
        scope_nonce: Option<u64>,
        switch_nonce: Option<u64>,
    ) {
        self.unicast(ServerMessage::ProtocolError {
            error,
            switch_nonce,
            scope_nonce,
        });
    }
}

#[cfg(test)]
#[path = "channel_test/mod.rs"]
mod tests;
