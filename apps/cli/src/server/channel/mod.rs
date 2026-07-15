// apps/cli/src/server/channel/mod.rs
//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! # 响应通道抽象 (Response Channel)
//!
//! **功能**:
//! 提供统一的双通道发送接口，区分广播 (Broadcast) 和单播 (Unicast)。

mod delivery;

pub(crate) use delivery::DeliveryOutcome;
use delivery::send_unicast;
use deve_core::protocol::{ServerError, ServerMessage};
use tokio::sync::{broadcast, mpsc, watch};

/// 双通道上下文
///
/// 同时持有广播和单播通道，供 Handler 按需选择。
#[derive(Clone)]
pub struct DualChannel {
    /// 广播通道 - 全局事件
    pub broadcast: broadcast::Sender<ServerMessage>,
    /// 单播通道 - 单客户端响应
    pub unicast: mpsc::Sender<ServerMessage>,
    diff_unicast: mpsc::Sender<ServerMessage>,
    retire_session: Option<watch::Sender<bool>>,
}

impl DualChannel {
    /// 创建双通道上下文
    pub fn new(
        broadcast: broadcast::Sender<ServerMessage>,
        unicast: mpsc::Sender<ServerMessage>,
    ) -> Self {
        let (retire_session, _) = watch::channel(false);
        Self {
            broadcast,
            diff_unicast: unicast.clone(),
            unicast,
            retire_session: Some(retire_session),
        }
    }

    /// Creates a channel with a dedicated one-slot path for large typed diff payloads.
    #[cfg(test)]
    pub(crate) fn with_diff_channel(
        broadcast: broadcast::Sender<ServerMessage>,
        unicast: mpsc::Sender<ServerMessage>,
        diff_unicast: mpsc::Sender<ServerMessage>,
    ) -> Self {
        let (retire_session, _) = watch::channel(false);
        Self {
            broadcast,
            unicast,
            diff_unicast,
            retire_session: Some(retire_session),
        }
    }

    #[cfg(test)]
    pub(crate) fn retirement_receiver(&self) -> watch::Receiver<bool> {
        self.retire_session
            .as_ref()
            .expect("test channel has retirement signal")
            .subscribe()
    }

    pub(crate) fn with_diff_channel_and_retirement(
        broadcast: broadcast::Sender<ServerMessage>,
        unicast: mpsc::Sender<ServerMessage>,
        diff_unicast: mpsc::Sender<ServerMessage>,
        retire_session: watch::Sender<bool>,
    ) -> Self {
        Self {
            broadcast,
            unicast,
            diff_unicast,
            retire_session: Some(retire_session),
        }
    }

    /// 广播消息 (全局事件)
    pub fn broadcast(&self, msg: ServerMessage) {
        let _ = self.broadcast.send(msg);
    }

    /// 单播消息 (单客户端响应)
    pub fn unicast(&self, msg: ServerMessage) {
        if send_unicast(&self.unicast, msg) == DeliveryOutcome::CriticalQueueFull
            && let Some(retire_session) = &self.retire_session
        {
            let _ = retire_session.send(true);
        }
    }

    pub(crate) fn diff_unicast_sender(&self) -> mpsc::Sender<ServerMessage> {
        self.diff_unicast.clone()
    }

    pub(crate) async fn diff_unicast(&self, message: ServerMessage) -> bool {
        self.diff_unicast.send(message).await.is_ok()
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
mod tests;
