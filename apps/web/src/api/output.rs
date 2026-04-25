// apps/web/src/api/output.rs
//! plan_ref:
//!   - 09_auth#unauthorized-disconnected-ui
//!
//! # WebSocket 输出管理器
//!
//! ## 职责
//! 1. 维护离线消息队列 (有容量上限)
//! 2. 过滤断连期间不允许缓存的写入类消息
//! 3. 提供发送失败时的重排队辅助
//!
//! ## 队列策略
//! 队列有 `MAX_QUEUE_SIZE` 上限。超过限制时丢弃最旧的消息并警告。
//! 这防止了网络断开时因持续操作导致的内存耗尽。

use deve_core::protocol::ClientMessage;
use deve_core::protocol::frame::encode_client_binary;
use leptos::prelude::*;
use std::collections::VecDeque;

use self::output_write::drop_queued_writes;
pub(crate) use self::output_write::is_write_message;
use super::ConnectionStatus;
use super::socket::BrowserSocket;
mod output_write;
#[cfg(test)]
mod tests;
/// 离线队列最大容量
/// 防止网络断开时内存无限增长
const MAX_QUEUE_SIZE: usize = 500;

pub(crate) fn prepare_queue_for_new_connection(queue: &mut VecDeque<ClientMessage>) {
    drop_queued_writes(queue);
    leptos::logging::log!("OutputLoop: 收到新连接。刷新 {} 条消息。", queue.len() + 1);
    queue.push_front(ClientMessage::Ping);
}

pub(crate) fn send_or_requeue(
    socket: &BrowserSocket,
    msg: ClientMessage,
    queue: &mut VecDeque<ClientMessage>,
    set_status: WriteSignal<ConnectionStatus>,
) -> bool {
    let bytes = match encode_client_binary(&msg) {
        Ok(bytes) => bytes,
        Err(e) => {
            leptos::logging::error!("消息序列化失败: {:?}, 消息: {:?}", e, msg);
            return true;
        }
    };

    if let Err(e) = socket.send_binary(&bytes) {
        leptos::logging::warn!("WS 发送错误: {:?}. 入队中...", e);
        enqueue_with_limit(queue, msg);
        set_status.set(ConnectionStatus::Disconnected);
        return false;
    }
    true
}

/// 带容量限制的入队操作
///
/// 如果队列已满，丢弃最旧的消息并警告
pub(crate) fn enqueue_with_limit(queue: &mut VecDeque<ClientMessage>, msg: ClientMessage) {
    if queue.len() >= MAX_QUEUE_SIZE {
        let dropped = queue.pop_front();
        leptos::logging::warn!(
            "离线队列已满 ({}), 丢弃最旧消息: {:?}",
            MAX_QUEUE_SIZE,
            dropped
        );
    }
    queue.push_back(msg);
}
