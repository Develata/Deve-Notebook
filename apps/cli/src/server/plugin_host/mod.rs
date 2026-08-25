// apps/cli/src/server/plugin_host/mod.rs
//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
//! # Plugin Host Only Server

mod routes;
mod ws_host;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::Notify;
use tokio::sync::broadcast;

use deve_core::plugin::runtime::PluginRuntime;
use deve_core::protocol::ServerMessage;

pub use ws_host::ws_handler;

#[derive(Clone)]
pub struct PluginHostState {
    pub plugins: Arc<Vec<Box<dyn PluginRuntime>>>,
    pub tx: broadcast::Sender<ServerMessage>,
    _native_ai_registration: Arc<crate::server::ai_chat::NativeAiRuntimeRegistration>,
    _ai_provider_settings_registration:
        Arc<crate::server::ai_chat::settings::ProviderSettingsRegistration>,
    lifecycle: Arc<PluginHostLifecycle>,
}

const PLUGIN_HOST_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default)]
struct PluginHostLifecycle {
    cancelled: AtomicBool,
    active_sessions: AtomicUsize,
    changed: Notify,
}

impl PluginHostLifecycle {
    fn begin_session(self: &Arc<Self>) -> Option<PluginHostSession> {
        if self.cancelled.load(Ordering::Acquire) {
            return None;
        }
        self.active_sessions.fetch_add(1, Ordering::AcqRel);
        if self.cancelled.load(Ordering::Acquire) {
            self.finish_session();
            return None;
        }
        Some(PluginHostSession {
            lifecycle: self.clone(),
        })
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.changed.notify_waiters();
    }

    async fn cancelled(&self) {
        loop {
            let changed = self.changed.notified();
            if self.cancelled.load(Ordering::Acquire) {
                return;
            }
            changed.await;
        }
    }

    async fn wait_idle(&self) {
        loop {
            let changed = self.changed.notified();
            if self.active_sessions.load(Ordering::Acquire) == 0 {
                return;
            }
            changed.await;
        }
    }

    fn finish_session(&self) {
        if self.active_sessions.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.changed.notify_waiters();
        }
    }
}

struct PluginHostSession {
    lifecycle: Arc<PluginHostLifecycle>,
}

impl Drop for PluginHostSession {
    fn drop(&mut self) {
        self.lifecycle.finish_session();
    }
}

fn build_plugin_host_state(
    plugins: Vec<Box<dyn PluginRuntime>>,
    tx: broadcast::Sender<ServerMessage>,
) -> anyhow::Result<Arc<PluginHostState>> {
    crate::server::runtime::activate_plugin_runtimes(&plugins)?;
    let native_ai_registration =
        Arc::new(crate::server::ai_chat::NativeAiRuntimeRegistration::from_plugins(&plugins));
    let provider_settings = Arc::new(
        crate::server::ai_chat::settings::NativeAiProviderSettingsRuntime::environment_only()?,
    );
    let provider_settings_registration = Arc::new(crate::server::ai_chat::settings::register(
        provider_settings,
    )?);
    Ok(Arc::new(PluginHostState {
        plugins: Arc::new(plugins),
        tx,
        _native_ai_registration: native_ai_registration,
        _ai_provider_settings_registration: provider_settings_registration,
        lifecycle: Arc::new(PluginHostLifecycle::default()),
    }))
}

pub async fn start_plugin_host_only(
    plugins: Vec<Box<dyn PluginRuntime>>,
    port: u16,
) -> anyhow::Result<()> {
    crate::server::ai_chat::init_chat_stream_handler()?;
    let (tx, _rx) = broadcast::channel(100);
    let state = build_plugin_host_state(plugins, tx)?;

    let lifecycle = state.lifecycle.clone();
    let app = routes::build_router(state);
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Plugin host running on ws://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    serve_until_shutdown(
        listener,
        app,
        lifecycle,
        crate::server::start::shutdown_signal::production_shutdown_signal(),
    )
    .await?;
    Ok(())
}

async fn serve_until_shutdown<F>(
    listener: tokio::net::TcpListener,
    app: axum::Router,
    lifecycle: Arc<PluginHostLifecycle>,
    shutdown: F,
) -> std::io::Result<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let shutdown_lifecycle = lifecycle.clone();
    let (shutdown_seen_tx, mut shutdown_seen_rx) = tokio::sync::oneshot::channel();
    let server = std::future::IntoFuture::into_future(
        axum::serve(listener, app).with_graceful_shutdown(async move {
            shutdown.await;
            shutdown_lifecycle.cancel();
            let _ = shutdown_seen_tx.send(());
        }),
    );
    tokio::pin!(server);

    tokio::select! {
        result = &mut server => return result,
        observed = &mut shutdown_seen_rx => {
            if observed.is_err() {
                return server.await;
            }
        }
    }

    let deadline = tokio::time::Instant::now() + PLUGIN_HOST_SHUTDOWN_TIMEOUT;
    tokio::time::timeout_at(deadline, &mut server)
        .await
        .map_err(|_| plugin_host_shutdown_timeout())??;
    tokio::time::timeout_at(deadline, lifecycle.wait_idle())
        .await
        .map_err(|_| plugin_host_shutdown_timeout())?;
    Ok(())
}

fn plugin_host_shutdown_timeout() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        "plugin host sessions did not retire before shutdown deadline",
    )
}

#[cfg(test)]
mod tests {
    use super::{build_plugin_host_state, serve_until_shutdown};
    use futures::StreamExt;
    use std::sync::Arc;

    #[test]
    fn proxy_host_owns_builtin_ai_registration_for_its_lifetime() {
        let plugins =
            crate::server::ai_chat::assemble_runtime_plugins_with_policy(Vec::new(), true)
                .expect("built-in runtime");
        let (tx, _) = tokio::sync::broadcast::channel(1);
        let state = build_plugin_host_state(plugins, tx).expect("plugin host state");

        assert!(state._native_ai_registration.is_registered());
        assert!(
            state
                .plugins
                .iter()
                .any(|plugin| plugin.manifest().id == "ai-chat")
        );
    }

    #[tokio::test]
    async fn plugin_host_shutdown_signal_retires_listener() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");

        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            serve_until_shutdown(
                listener,
                axum::Router::new(),
                Arc::new(super::PluginHostLifecycle::default()),
                std::future::ready(()),
            ),
        )
        .await
        .expect("shutdown deadline")
        .expect("graceful shutdown");
    }

    #[tokio::test]
    async fn plugin_host_shutdown_retires_active_websocket_session() {
        let (broadcast, _) = tokio::sync::broadcast::channel(4);
        let state = build_plugin_host_state(Vec::new(), broadcast).expect("plugin host state");
        let lifecycle = state.lifecycle.clone();
        let app = super::routes::build_router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let address = listener.local_addr().expect("listener address");
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            serve_until_shutdown(listener, app, lifecycle, async move {
                let _ = shutdown_rx.await;
            })
            .await
        });
        let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{address}/ws"))
            .await
            .expect("active websocket");

        shutdown_tx.send(()).expect("shutdown signal");
        tokio::time::timeout(std::time::Duration::from_secs(1), server)
            .await
            .expect("server shutdown deadline")
            .expect("server task")
            .expect("server shutdown");
        let terminal = tokio::time::timeout(std::time::Duration::from_secs(1), socket.next())
            .await
            .expect("websocket retirement deadline");
        assert!(
            !matches!(
                terminal,
                Some(Ok(tokio_tungstenite::tungstenite::Message::Text(_)))
            ),
            "retired websocket must not remain an active text session"
        );
    }
}
