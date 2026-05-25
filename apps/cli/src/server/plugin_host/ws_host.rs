//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
use super::PluginHostState;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::StreamExt;
use std::sync::Arc;

use crate::server::channel::DualChannel;
use crate::server::handlers::plugin::handle_plugin_call_with_plugins;
use crate::server::ws::send;
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
    send::spawn_unicast_sender_task(sender, unicast_rx);

    let broadcast_rx = state.tx.subscribe();
    send::spawn_broadcast_forwarder(
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

        if let axum::extract::ws::Message::Text(text) = msg {
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(ClientMessage::PluginCall {
                    req_id,
                    plugin_id,
                    fn_name,
                    args,
                }) => {
                    handle_plugin_call_with_plugins(
                        state.plugins.as_ref(),
                        &ch,
                        req_id,
                        plugin_id,
                        fn_name,
                        args,
                    )
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
        }
    }
}
