// apps/cli/src/server/plugin_host.rs
//! # Plugin Host Only Server

use axum::Router;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::server::channel::DualChannel;
use crate::server::handlers::plugin::handle_plugin_call_with_plugins;
use crate::server::handlers::{repo, source_control};
use crate::server::node_role_http;
use crate::server::ws::send;
use deve_core::plugin::runtime::PluginRuntime;
use deve_core::protocol::{ClientMessage, ServerMessage};

#[derive(Clone)]
pub struct PluginHostState {
    pub plugins: Arc<Vec<Box<dyn PluginRuntime>>>,
    pub tx: broadcast::Sender<ServerMessage>,
}

pub async fn start_plugin_host_only(
    plugins: Vec<Box<dyn PluginRuntime>>,
    port: u16,
) -> anyhow::Result<()> {
    crate::server::ai_chat::init_chat_stream_handler()?;
    let (tx, _rx) = broadcast::channel(100);
    let state = Arc::new(PluginHostState {
        plugins: Arc::new(plugins),
        tx,
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/repo/docs", get(repo::http::list_docs_plugin_host))
        .route("/api/repo/doc", get(repo::http::doc_content_plugin_host))
        .route(
            "/api/sc/status",
            get(source_control::http::status_plugin_host),
        )
        .route("/api/sc/diff", get(source_control::http::diff_plugin_host))
        .route(
            "/api/sc/stage",
            post(source_control::http::stage_plugin_host),
        )
        .route(
            "/api/sc/commit",
            post(source_control::http::commit_plugin_host),
        )
        .route("/api/node/role", get(node_role_http::role))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Plugin host running on ws://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

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
    send::spawn_broadcast_forwarder(broadcast_rx, unicast_tx.clone());
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
                    ch.unicast(ServerMessage::Error(
                        "Plugin host only: unsupported message".into(),
                    ));
                }
                Err(_) => {
                    ch.unicast(ServerMessage::Error("Invalid client message".into()));
                }
            }
        }
    }
}
