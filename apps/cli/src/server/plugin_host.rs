// apps/cli/src/server/plugin_host.rs
//! # Plugin Host Only Server

#[path = "plugin_host_routes.rs"]
mod routes;
#[path = "plugin_host_ws.rs"]
mod ws_host;

use std::sync::Arc;
use tokio::sync::broadcast;

use deve_core::plugin::runtime::PluginRuntime;
use deve_core::protocol::ServerMessage;

pub use ws_host::ws_handler;

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

    let app = routes::build_router(state);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Plugin host running on ws://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
