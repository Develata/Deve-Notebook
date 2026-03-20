pub(crate) use super::filter::BroadcastFilter;
use crate::server::channel::try_send_with_delivery_class;
use axum::extract::ws::{Message, WebSocket};
use deve_core::protocol::{ServerError, ServerErrorCode, ServerMessage};
use futures::{Sink, SinkExt};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, mpsc};

/// 单播队列容量（每个连接）。
///
/// 目标：为慢客户端提供背压，避免无界内存增长。
pub(crate) const UNICAST_CAPACITY: usize = 256;

/// 创建有界单播通道。
pub(crate) fn new_unicast_channel() -> (mpsc::Sender<ServerMessage>, mpsc::Receiver<ServerMessage>)
{
    mpsc::channel(UNICAST_CAPACITY)
}

/// 启动单播发送任务：将单播队列中的消息写入 WebSocket。
///
/// ## 协议策略
/// - **使用二进制 (Bincode)**: 体积更小，解析更快，减少带宽占用。
pub(crate) fn spawn_unicast_sender_task(
    sender: futures::stream::SplitSink<WebSocket, Message>,
    rx: mpsc::Receiver<ServerMessage>,
) {
    spawn_unicast_sender_task_with_encoder(sender, rx, encode_server_message);
}

fn spawn_unicast_sender_task_with_encoder<S, E>(
    mut sender: S,
    mut rx: mpsc::Receiver<ServerMessage>,
    encode: E,
) where
    S: Sink<Message> + Unpin + Send + 'static,
    S::Error: std::fmt::Debug + Send + 'static,
    E: Fn(&ServerMessage) -> Result<Vec<u8>, String> + Send + 'static,
{
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let bytes = match encode(&msg) {
                Ok(bytes) => bytes,
                Err(err) => {
                    tracing::error!("Failed to serialize WS message; closing sender: {err}");
                    break;
                }
            };

            if let Err(e) = sender.send(Message::Binary(bytes)).await {
                tracing::debug!("WS sender closed while sending message: {:?}", e);
                break;
            }
        }
    });
}

fn encode_server_message(msg: &ServerMessage) -> Result<Vec<u8>, String> {
    bincode::serialize(msg).map_err(|err| err.to_string())
}

/// 启动广播转发任务：将广播消息尝试写入单播队列。
pub(crate) fn spawn_broadcast_forwarder(
    mut broadcast_rx: broadcast::Receiver<ServerMessage>,
    unicast_tx: mpsc::Sender<ServerMessage>,
    filter: BroadcastFilter,
) {
    tokio::spawn(async move {
        loop {
            match broadcast_rx.recv().await {
                Ok(msg) => {
                    let Some(msg) = filter.stamp_scope_nonce(msg) else {
                        continue;
                    };
                    if !filter.should_forward(&msg) {
                        continue;
                    }
                    let must_deliver = must_deliver_broadcast(&msg);
                    try_send_with_delivery_class(&unicast_tx, msg, must_deliver);
                }
                Err(RecvError::Lagged(skipped)) => {
                    tracing::warn!("WS broadcast lagged; skipped {} messages", skipped);
                    let error = ServerError::with_detail(
                        ServerErrorCode::RequestFailed,
                        format!(
                            "WS broadcast lagged; skipped {skipped} messages; reconnect or reopen to resync"
                        ),
                    );
                    let Some(msg) = filter.scoped_protocol_error(error, None) else {
                        continue;
                    };
                    try_send_with_delivery_class(&unicast_tx, msg, true);
                }
                Err(RecvError::Closed) => break,
            }
        }
    });
}

fn must_deliver_broadcast(msg: &ServerMessage) -> bool {
    matches!(
        msg,
        ServerMessage::FsChangeDetected { .. }
            | ServerMessage::CommitAck { .. }
            | ServerMessage::MergeComplete { .. }
            | ServerMessage::NewOp { .. }
            | ServerMessage::PeerDeleted { .. }
    )
}

#[cfg(test)]
#[path = "send_test.rs"]
mod tests;
