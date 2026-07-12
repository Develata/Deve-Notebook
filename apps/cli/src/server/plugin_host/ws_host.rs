//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
use super::PluginHostState;
use axum::extract::ws::Message;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::StreamExt;
use std::sync::Arc;

use crate::server::channel::DualChannel;
use crate::server::handlers::plugin::handle_plugin_call_with_plugins;
use crate::server::ws::send;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::protocol::{ClientMessage, ServerError, ServerErrorCode};

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<PluginHostState>>,
) -> impl IntoResponse {
    let peer_id = uuid::Uuid::new_v4().to_string();
    ws.on_upgrade(move |socket| handle_socket(state, socket, peer_id))
}

async fn handle_socket(
    state: Arc<PluginHostState>,
    socket: axum::extract::ws::WebSocket,
    peer_id: String,
) {
    let (sender, mut receiver) = socket.split();
    let (unicast_tx, unicast_rx) = send::new_unicast_channel();
    let unicast_task = send::spawn_unicast_sender_task(sender, unicast_rx);

    let broadcast_rx = state.tx.subscribe();
    let broadcast_task = send::spawn_broadcast_forwarder(
        broadcast_rx,
        unicast_tx.clone(),
        send::BroadcastFilter::allow_all(),
    );
    let ch = DualChannel::new(state.tx.clone(), unicast_tx);
    tracing::info!("Plugin host client connected: {}", peer_id);

    while let Some(msg) = receiver.next().await {
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!("WS receive error: {:?}", e);
                break;
            }
        };

        if !handle_client_message(state.plugins.as_ref(), &ch, msg).await {
            break;
        }
    }
    broadcast_task.abort();
    let _ = broadcast_task.await;
    unicast_task.abort();
    let _ = unicast_task.await;
}

async fn handle_client_message(
    plugins: &[Box<dyn PluginRuntime>],
    ch: &DualChannel,
    msg: Message,
) -> bool {
    match msg {
        Message::Text(text) => {
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::PluginCall {
                    req_id,
                    plugin_id,
                    fn_name,
                    args,
                }) => {
                    handle_plugin_call_with_plugins(plugins, ch, req_id, plugin_id, fn_name, args)
                        .await;
                }
                Ok(_) => {
                    ch.send_protocol_error(ServerError::new(
                        ServerErrorCode::PluginUnsupportedMessage,
                    ));
                }
                Err(_) => {
                    ch.send_protocol_error(ServerError::new(ServerErrorCode::PluginInvalidMessage));
                }
            }
            true
        }
        Message::Binary(_) => {
            ch.send_protocol_error(ServerError::new(ServerErrorCode::PluginInvalidMessage));
            true
        }
        Message::Ping(_) | Message::Pong(_) => true,
        Message::Close(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::handle_client_message;
    use crate::server::channel::DualChannel;
    use axum::extract::ws::Message;
    use deve_core::plugin::runtime::PluginRuntime;
    use deve_core::protocol::{ServerErrorCode, ServerMessage};
    use tokio::sync::{broadcast, mpsc};

    #[tokio::test]
    async fn binary_plugin_host_message_fails_closed() {
        let plugins: Vec<Box<dyn PluginRuntime>> = Vec::new();
        let (tx, _) = broadcast::channel(4);
        let (uni_tx, mut uni_rx) = mpsc::channel(4);
        let ch = DualChannel::new(tx, uni_tx);

        assert!(handle_client_message(&plugins, &ch, Message::Binary(vec![1, 2, 3])).await);

        match uni_rx.recv().await.expect("protocol error") {
            ServerMessage::ProtocolError { error, .. } => {
                assert_eq!(error.code, ServerErrorCode::PluginInvalidMessage);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }
}
