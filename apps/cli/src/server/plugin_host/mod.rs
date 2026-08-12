// apps/cli/src/server/plugin_host/mod.rs
//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
//! # Plugin Host Only Server

mod routes;
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
    _native_ai_registration: Arc<crate::server::ai_chat::NativeAiRuntimeRegistration>,
}

fn build_plugin_host_state(
    plugins: Vec<Box<dyn PluginRuntime>>,
    tx: broadcast::Sender<ServerMessage>,
) -> Arc<PluginHostState> {
    let native_ai_registration =
        Arc::new(crate::server::ai_chat::NativeAiRuntimeRegistration::from_plugins(&plugins));
    Arc::new(PluginHostState {
        plugins: Arc::new(plugins),
        tx,
        _native_ai_registration: native_ai_registration,
    })
}

pub async fn start_plugin_host_only(
    plugins: Vec<Box<dyn PluginRuntime>>,
    port: u16,
) -> anyhow::Result<()> {
    crate::server::ai_chat::init_chat_stream_handler()?;
    let (tx, _rx) = broadcast::channel(100);
    let state = build_plugin_host_state(plugins, tx);

    let app = routes::build_router(state);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Plugin host running on ws://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build_plugin_host_state;

    #[test]
    fn proxy_host_owns_builtin_ai_registration_for_its_lifetime() {
        let plugins =
            crate::server::ai_chat::assemble_runtime_plugins_with_policy(Vec::new(), true)
                .expect("built-in runtime");
        let (tx, _) = tokio::sync::broadcast::channel(1);
        let state = build_plugin_host_state(plugins, tx);

        assert!(state._native_ai_registration.is_registered());
        assert!(
            state
                .plugins
                .iter()
                .any(|plugin| plugin.manifest().id == "ai-chat")
        );
    }
}
