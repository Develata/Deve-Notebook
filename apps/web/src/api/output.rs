// apps/web/src/api/output.rs
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
use leptos::prelude::*;
use std::collections::VecDeque;

use super::ConnectionStatus;
use super::socket::BrowserSocket;
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
    let text = match serde_json::to_string(&msg) {
        Ok(t) => t,
        Err(e) => {
            leptos::logging::error!("消息序列化失败: {:?}, 消息: {:?}", e, msg);
            return true;
        }
    };

    if let Err(e) = socket.send_text(&text) {
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

fn drop_queued_writes(queue: &mut VecDeque<ClientMessage>) {
    queue.retain(|msg| !is_write_message(msg));
}

/// 判断消息是否为写入类操作
///
/// WebLightPeer 约束：断连时禁止写入，只允许查询类消息。
pub(crate) fn is_write_message(msg: &ClientMessage) -> bool {
    match msg {
        // 编辑操作
        ClientMessage::Edit { .. } => true,
        ClientMessage::CreateDoc { .. } => true,
        ClientMessage::RenameDoc { .. } => true,
        ClientMessage::DeleteDoc { .. } => true,
        ClientMessage::CopyDoc { .. } => true,
        ClientMessage::MoveDoc { .. } => true,
        // 同步操作
        ClientMessage::SyncPush { .. } => true,
        ClientMessage::SyncPushSnapshot { .. } => true,
        // 版本控制操作
        ClientMessage::Commit { .. } => true,
        ClientMessage::CommitAndPush { .. } => true,
        ClientMessage::StageFile { .. } => true,
        ClientMessage::StageFiles { .. } => true,
        ClientMessage::UnstageFile { .. } => true,
        ClientMessage::UnstageFiles { .. } => true,
        ClientMessage::DiscardFile { .. } => true,
        ClientMessage::ResolveConflict { .. } => true,
        // 分支操作
        ClientMessage::DeletePeer { .. } => true,
        ClientMessage::SwitchBranch { .. } => true,
        ClientMessage::SwitchRepo { .. } => true,
        ClientMessage::SwitchRepoExact { .. } => true,
        // 合并操作
        ClientMessage::ConfirmMerge { .. } => true,
        ClientMessage::DiscardPending { .. } => true,
        ClientMessage::SetSyncMode { .. } => true,
        // 插件调用视为写入（可能有副作用）
        ClientMessage::PluginCall { .. } => true,
        // 其他为查询类消息
        _ => false,
    }
}
