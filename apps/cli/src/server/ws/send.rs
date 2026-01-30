use axum::extract::ws::{Message, WebSocket};
use deve_core::protocol::ServerMessage;
use futures::SinkExt;
use tokio::sync::{broadcast, mpsc};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc::error::TrySendError;

/// 单播队列容量（每个连接）。
///
/// 目标：为慢客户端提供背压，避免无界内存增长。
pub(crate) const UNICAST_CAPACITY: usize = 256;

/// 创建有界单播通道。
pub(crate) fn new_unicast_channel() -> (mpsc::Sender<ServerMessage>, mpsc::Receiver<ServerMessage>) {
    mpsc::channel(UNICAST_CAPACITY)
}

/// 启动单播发送任务：将单播队列中的消息写入 WebSocket。
///
/// ## 协议策略
/// - **使用二进制 (Bincode)**: 体积更小，解析更快，减少带宽占用。
pub(crate) fn spawn_unicast_sender_task(
    mut sender: futures::stream::SplitSink<WebSocket, Message>,
    mut rx: mpsc::Receiver<ServerMessage>,
) {
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let bytes = match bincode::serialize(&msg) {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("Failed to serialize WS message: {:?}", e);
                    continue;
                }
            };

            if let Err(e) = sender.send(Message::Binary(bytes)).await {
                tracing::warn!("Failed to send message to WS: {:?}", e);
                break;
            }
        }
    });
}

/// 启动广播转发任务：将广播消息尝试写入单播队列。
pub(crate) fn spawn_broadcast_forwarder(
    mut broadcast_rx: broadcast::Receiver<ServerMessage>,
    unicast_tx: mpsc::Sender<ServerMessage>,
) {
    tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(msg) => {
                    if let Err(e) = unicast_tx.try_send(msg) {
                        match e {
                            TrySendError::Full(_) => {
                                tracing::warn!("WS unicast queue full; dropping broadcast");
                            }
                            TrySendError::Closed(_) => break,
                        }
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    tracing::warn!("WS broadcast lagged; skipped {} messages", skipped);
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

