// apps/web/src/api/output.rs
//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 08_auth#unauthorized-disconnected-ui
//!
//! # WebSocket 输出管理器
//!
//! ## 职责
//! 1. 维护离线消息队列 (有容量上限)
//! 2. 过滤断连期间不允许缓存的写入类消息
//! 3. 提供发送失败时的重排队辅助
//!
//! ## 队列策略
//! 队列有 `MAX_QUEUE_SIZE` 上限。容量耗尽时拒绝新消息并退休当前
//! connection generation；绝不淘汰已经 admission 的旧消息。

use std::collections::VecDeque;

#[cfg(test)]
pub(crate) use self::output_write::is_write_message;
pub(crate) use self::output_write::{OutboundMessageClass, classify_outbound_message};
use super::outbound_admission::OutboundFrame;
use super::socket::BrowserSocket;
mod output_write;
#[cfg(test)]
mod tests;
/// 离线队列最大容量
/// 防止网络断开时内存无限增长
const MAX_QUEUE_SIZE: usize = super::outbound_admission::OUTBOUND_ADMISSION_LIMIT + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SocketDispatchFailure {
    Transport,
}

pub(crate) fn prepare_queue_for_new_connection(queue: &mut VecDeque<OutboundFrame>) {
    queue.retain(|frame| frame.message_class() == OutboundMessageClass::Read);
    if queue.len() < MAX_QUEUE_SIZE {
        match OutboundFrame::system_ping() {
            Ok(ping) => queue.push_front(ping),
            Err(_) => leptos::logging::error!(
                "web_socket_system_ping_encode_rejected error_category=protocol_encode"
            ),
        }
    } else {
        leptos::logging::warn!(
            "web_socket_queue_system_ping_skipped category=queue_saturated limit={}",
            MAX_QUEUE_SIZE
        );
    }
    leptos::logging::log!("OutputLoop: 收到新连接。刷新 {} 条消息。", queue.len());
}

pub(crate) fn send_or_requeue(
    socket: &BrowserSocket,
    frame: OutboundFrame,
    queue: &mut VecDeque<OutboundFrame>,
) -> Result<(), SocketDispatchFailure> {
    let message_class = frame.message_class();

    if socket.send_binary(frame.bytes()).is_err() {
        leptos::logging::warn!(
            "web_socket_send_failed class={} action=requeue_front_and_retire",
            message_class.label()
        );
        requeue_front_after_send_failure(queue, frame);
        return Err(SocketDispatchFailure::Transport);
    }
    Ok(())
}

/// 带容量限制的入队操作
///
/// 如果队列已满，保留现有顺序并把新消息返回给调用者。
pub(crate) fn try_enqueue(
    queue: &mut VecDeque<OutboundFrame>,
    frame: OutboundFrame,
) -> Result<(), OutboundFrame> {
    if queue.len() >= MAX_QUEUE_SIZE {
        return Err(frame);
    }
    queue.push_back(frame);
    Ok(())
}

fn requeue_front_after_send_failure(queue: &mut VecDeque<OutboundFrame>, frame: OutboundFrame) {
    debug_assert!(queue.len() < MAX_QUEUE_SIZE);
    queue.push_front(frame);
}
