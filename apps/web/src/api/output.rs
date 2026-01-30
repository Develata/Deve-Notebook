// apps/web/src/api/output.rs
//! # WebSocket 输出管理器
//!
//! ## 职责
//! 1. 接收应用层消息
//! 2. 维护离线消息队列 (有容量上限)
//! 3. 新连接建立时刷新队列
//! 4. 向服务器发送消息
//!
//! ## 队列策略
//! 队列有 `MAX_QUEUE_SIZE` 上限。超过限制时丢弃最旧的消息并警告。
//! 这防止了网络断开时因持续操作导致的内存耗尽。

use deve_core::protocol::ClientMessage;
use futures::channel::mpsc::UnboundedReceiver;
use futures::stream::SplitSink;
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::{Message, futures::WebSocket};
use leptos::task::spawn_local;
use std::collections::VecDeque;

/// 离线队列最大容量
/// 防止网络断开时内存无限增长
const MAX_QUEUE_SIZE: usize = 500;

/// 输出管理器循环的内部消息类型
pub enum OutputEvent {
    /// 应用程序要发送到服务器的消息
    Client(Box<ClientMessage>),
    /// 来自成功连接的新 WebSocket 写入端
    NewLink(SplitSink<WebSocket, Message>),
}

/// 启动输出管理器任务
pub fn spawn_output_manager(
    rx: UnboundedReceiver<ClientMessage>,
    link_rx: UnboundedReceiver<SplitSink<WebSocket, Message>>,
) {
    spawn_local(async move {
        let mut current_sink: Option<SplitSink<WebSocket, Message>> = None;
        let mut queue: VecDeque<ClientMessage> = VecDeque::new();

        // 合并流: 客户端消息 + 新连接链接
        let mut events = futures::stream::select(
            rx.map(|m| OutputEvent::Client(Box::new(m))),
            link_rx.map(OutputEvent::NewLink),
        );

        while let Some(event) = events.next().await {
            match event {
                OutputEvent::NewLink(sink) => {
                    handle_new_link(sink, &mut current_sink, &mut queue).await;
                }
                OutputEvent::Client(msg) => {
                    handle_client_message(*msg, &mut current_sink, &mut queue).await;
                }
            }
        }
    });
}

/// 处理新的 WebSocket 连接链接
async fn handle_new_link(
    sink: SplitSink<WebSocket, Message>,
    current_sink: &mut Option<SplitSink<WebSocket, Message>>,
    queue: &mut VecDeque<ClientMessage>,
) {
    leptos::logging::log!("OutputLoop: 收到新连接。刷新 {} 条消息。", queue.len() + 1);
    *current_sink = Some(sink);

    // 注入 ListDocs 到队首以刷新状态
    queue.push_front(ClientMessage::ListDocs);

    // 刷新队列
    flush_queue(current_sink, queue).await;
}

/// 处理要发送的客户端消息
///
/// ## 协议策略
/// - **使用二进制 (Bincode)**: 体积更小，解析更快。
async fn handle_client_message(
    msg: ClientMessage,
    current_sink: &mut Option<SplitSink<WebSocket, Message>>,
    queue: &mut VecDeque<ClientMessage>,
) {
    if let Some(writer) = current_sink.as_mut() {
        let bytes = match bincode::serialize(&msg) {
            Ok(b) => b,
            Err(e) => {
                leptos::logging::error!("消息序列化失败: {:?}, 消息: {:?}", e, msg);
                return;
            }
        };

        if let Err(e) = writer.send(Message::Bytes(bytes)).await {
            leptos::logging::warn!("WS 发送错误: {:?}. 入队中...", e);
            enqueue_with_limit(queue, msg);
            *current_sink = None; // 标记连接已死
        }
    } else {
        // 离线状态，入队等待
        enqueue_with_limit(queue, msg);
    }
}

/// 带容量限制的入队操作
///
/// 如果队列已满，丢弃最旧的消息并警告
fn enqueue_with_limit(queue: &mut VecDeque<ClientMessage>, msg: ClientMessage) {
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

/// 将队列中的所有消息刷新到当前连接
///
/// ## 协议策略
/// - **使用二进制 (Bincode)**: 体积更小，解析更快。
async fn flush_queue(
    current_sink: &mut Option<SplitSink<WebSocket, Message>>,
    queue: &mut VecDeque<ClientMessage>,
) {
    let count = queue.len();
    for _ in 0..count {
        if let Some(msg) = queue.pop_front() {
            let bytes = match bincode::serialize(&msg) {
                Ok(b) => b,
                Err(e) => {
                    leptos::logging::error!("刷新队列时序列化失败: {:?}", e);
                    continue;
                }
            };

            let writer = match current_sink.as_mut() {
                Some(w) => w,
                None => {
                    queue.push_front(msg);
                    break;
                }
            };

            if let Err(e) = writer.send(Message::Bytes(bytes)).await {
                leptos::logging::error!("WS 刷新错误: {:?}. 连接可能已断开。", e);
                queue.push_front(msg);
                *current_sink = None;
                break;
            }
        }
    }
}
