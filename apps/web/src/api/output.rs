// apps\web\src\api
//! # WebSocket 输出管理器
//!
//! ## 职责
//! 1. 接收应用层消息
//! 2. 维护离线消息队列
//! 3. 新连接建立时刷新队列
//! 4. 向服务器发送消息

use leptos::task::spawn_local;
use gloo_net::websocket::{futures::WebSocket, Message};
use futures::{StreamExt, SinkExt};
use futures::channel::mpsc::UnboundedReceiver;
use futures::stream::SplitSink;
use std::collections::VecDeque;
use deve_core::protocol::ClientMessage;

/// 输出管理器循环的内部消息类型
pub enum OutputEvent {
    /// 应用程序要发送到服务器的消息
    Client(ClientMessage),
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
        
        // Merge streams: Client Messages + New Connection Links
        let mut events = futures::stream::select(
            rx.map(OutputEvent::Client),
            link_rx.map(OutputEvent::NewLink)
        );
        
        while let Some(event) = events.next().await {
            match event {
                OutputEvent::NewLink(sink) => {
                    handle_new_link(sink, &mut current_sink, &mut queue).await;
                },
                OutputEvent::Client(msg) => {
                    handle_client_message(msg, &mut current_sink, &mut queue).await;
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
    leptos::logging::log!("OutputLoop: New Connection Link received. Flushing {} items.", queue.len() + 1);
    *current_sink = Some(sink);
    
    // Inject ListDocs at front to refresh state
    queue.push_front(ClientMessage::ListDocs);
    
    // Flush queue
    flush_queue(current_sink, queue).await;
}

/// 处理要发送的客户端消息
async fn handle_client_message(
    msg: ClientMessage,
    current_sink: &mut Option<SplitSink<WebSocket, Message>>,
    queue: &mut VecDeque<ClientMessage>,
) {
    if let Some(writer) = current_sink.as_mut() {
        let json = match serde_json::to_string(&msg) {
            Ok(j) => j,
            Err(_) => return,
        };
        
        if let Err(e) = writer.send(Message::Text(json)).await {
            leptos::logging::warn!("WS Send Error: {:?}. Queuing...", e);
            queue.push_back(msg);
            *current_sink = None; // Mark as dead
        }
    } else {
        // Offline, just queue
        queue.push_back(msg);
    }
}

/// 将队列中的所有消息刷新到当前连接
async fn flush_queue(
    current_sink: &mut Option<SplitSink<WebSocket, Message>>,
    queue: &mut VecDeque<ClientMessage>,
) {
    let count = queue.len();
    for _ in 0..count {
        if let Some(msg) = queue.pop_front() {
            let json = match serde_json::to_string(&msg) {
                Ok(j) => j,
                Err(_) => continue,
            };
            
            let writer = match current_sink.as_mut() {
                Some(w) => w,
                None => {
                    queue.push_front(msg);
                    break;
                }
            };
            
            if let Err(e) = writer.send(Message::Text(json)).await {
                leptos::logging::error!("WS Flush Error: {:?}. Connection likely died.", e);
                queue.push_front(msg);
                *current_sink = None;
                break;
            }
        }
    }
}
