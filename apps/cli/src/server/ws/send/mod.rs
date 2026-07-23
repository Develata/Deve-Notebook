//! plan_ref:
//!   - 07_network#server-ws-runtime
//!
//! WebSocket outbound broadcast and unicast delivery runtime.

pub(crate) use super::filter::BroadcastFilter;
use axum::extract::ws::{Message, WebSocket};
use deve_core::protocol::ServerMessage;
use deve_core::protocol::frame::encode_server_binary;
use futures::{Sink, SinkExt};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::{broadcast, mpsc};

use crate::server::metrics;

/// 单播队列容量（每个连接）。
///
/// 目标：为慢客户端提供背压，避免无界内存增长。
pub(crate) const UNICAST_CAPACITY: usize = 256;
pub(crate) const DIFF_UNICAST_CAPACITY: usize = 1;

/// 创建有界单播通道。
pub(crate) fn new_unicast_channel() -> (mpsc::Sender<ServerMessage>, mpsc::Receiver<ServerMessage>)
{
    mpsc::channel(UNICAST_CAPACITY)
}

pub(crate) fn new_diff_unicast_channel()
-> (mpsc::Sender<ServerMessage>, mpsc::Receiver<ServerMessage>) {
    mpsc::channel(DIFF_UNICAST_CAPACITY)
}

/// 启动单播发送任务：将单播队列中的消息写入 WebSocket。
///
/// ## 协议策略
/// - **使用二进制帧**: 体积更小，解析更快，减少带宽占用。
pub(crate) fn spawn_unicast_sender_task(
    sender: futures::stream::SplitSink<WebSocket, Message>,
    rx: mpsc::Receiver<ServerMessage>,
    diff_rx: mpsc::Receiver<ServerMessage>,
) -> tokio::task::JoinHandle<()> {
    spawn_unicast_sender_task_with_encoder(sender, rx, diff_rx, encode_server_message)
}

fn spawn_unicast_sender_task_with_encoder<S, E>(
    mut sender: S,
    mut rx: mpsc::Receiver<ServerMessage>,
    mut diff_rx: mpsc::Receiver<ServerMessage>,
    encode: E,
) -> tokio::task::JoinHandle<()>
where
    S: Sink<Message> + Unpin + Send + 'static,
    S::Error: std::fmt::Debug + Send + 'static,
    E: Fn(&ServerMessage) -> Result<Vec<u8>, String> + Send + 'static,
{
    tokio::spawn(async move {
        let mut regular_open = true;
        let mut diff_open = true;
        while regular_open || diff_open {
            let msg = tokio::select! {
                biased;
                msg = diff_rx.recv(), if diff_open => {
                    match msg {
                        Some(msg) => msg,
                        None => { diff_open = false; continue; }
                    }
                }
                msg = rx.recv(), if regular_open => {
                    match msg {
                        Some(msg) => msg,
                        None => { regular_open = false; continue; }
                    }
                }
            };
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
    })
}

fn encode_server_message(msg: &ServerMessage) -> Result<Vec<u8>, String> {
    encode_server_binary(msg).map_err(|err| err.to_string())
}

/// 启动广播转发任务：将广播消息尝试写入单播队列。
pub(crate) fn spawn_broadcast_forwarder(
    mut broadcast_rx: broadcast::Receiver<ServerMessage>,
    unicast_tx: mpsc::Sender<ServerMessage>,
    filter: BroadcastFilter,
) -> tokio::task::JoinHandle<()> {
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
                    if must_deliver {
                        if unicast_tx.send(msg).await.is_err() {
                            break;
                        }
                    } else {
                        match unicast_tx.try_send(msg) {
                            Ok(()) => {}
                            Err(TrySendError::Full(_)) => {
                                metrics::record_noncritical_broadcast_drop();
                                let counters = metrics::delivery_metrics_snapshot();
                                tracing::warn!(
                                    noncritical_broadcast_drops =
                                        counters.noncritical_broadcast_drops,
                                    "Dropping non-critical broadcast for slow WS session"
                                );
                            }
                            Err(TrySendError::Closed(_)) => break,
                        }
                    }
                }
                Err(RecvError::Lagged(skipped)) => {
                    metrics::record_broadcast_lag();
                    let counters = metrics::delivery_metrics_snapshot();
                    tracing::warn!(
                        skipped,
                        broadcast_lag_events = counters.broadcast_lag_events,
                        "WS broadcast lagged; scheduling scoped recovery"
                    );
                    let Some(msg) = filter.scoped_broadcast_gap_recovery(skipped) else {
                        continue;
                    };
                    metrics::record_broadcast_recovery();
                    let counters = metrics::delivery_metrics_snapshot();
                    tracing::debug!(
                        broadcast_recoveries = counters.broadcast_recoveries,
                        "Enqueuing scoped recovery after broadcast lag"
                    );
                    if unicast_tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Closed) => break,
            }
        }
    })
}

fn must_deliver_broadcast(msg: &ServerMessage) -> bool {
    matches!(
        msg,
        ServerMessage::FsChangeDetected { .. }
            | ServerMessage::CommitAck { .. }
            | ServerMessage::MergeComplete { .. }
            | ServerMessage::NewOp { .. }
            | ServerMessage::ProjectionRecoveryRequired(_)
            | ServerMessage::PeerDeleted { .. }
            | ServerMessage::RepoList { .. }
    )
}

#[cfg(test)]
mod tests;
