//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Shared end-to-end WebSocket protocol acceptance harness.

use super::{router, sync_hello_test_support::build_state};
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use deve_core::protocol::ServerMessage;
use deve_core::protocol::frame::{
    WS_FRAME_MAGIC, WS_PROTOCOL_VERSION, decode_client_binary_frame, decode_server_binary_frame,
    encode_client_binary,
};
use deve_core::security::AuthConfig;
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

pub(super) struct WsHarness {
    _dir: TempDir,
    pub(super) repo_id: uuid::Uuid,
    pub(super) local_peer_id: PeerId,
    ws_url: String,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl WsHarness {
    pub(super) async fn spawn() -> anyhow::Result<Self> {
        let (dir, state, repo_id) = build_state()?;
        let local_peer_id = state.identity_key.peer_id();
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let mut auth_config = AuthConfig::dev_default()?;
        auth_config.allow_anonymous_localhost = true;
        let app = router::build_app(state, addr.port(), Arc::new(auth_config))?
            .into_make_service_with_connect_info::<SocketAddr>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("serve ws protocol harness");
        });
        Ok(Self {
            _dir: dir,
            repo_id,
            local_peer_id,
            ws_url: format!("ws://{addr}/ws"),
            shutdown: Some(shutdown_tx),
            task,
        })
    }

    pub(super) async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.task.await;
    }
}

pub(super) async fn connect_harness(
    harness: &WsHarness,
) -> anyhow::Result<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let (ws, _response) = connect_async(&harness.ws_url).await?;
    Ok(ws)
}

pub(super) async fn send_client_message<S>(ws: &mut S, msg: ClientMessage) -> anyhow::Result<()>
where
    S: Sink<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let bytes = encode_client_binary(&msg)?;
    assert_current_client_frame(&bytes)?;
    ws.send(Message::Binary(bytes)).await?;
    Ok(())
}

pub(super) async fn recv_server_message<S>(ws: &mut S) -> anyhow::Result<ServerMessage>
where
    S: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    let msg = timeout(Duration::from_secs(2), ws.next()).await?;
    let Some(msg) = msg else {
        anyhow::bail!("websocket closed before server response");
    };
    let Message::Binary(bytes) = msg? else {
        anyhow::bail!("expected binary server frame");
    };
    assert!(bytes.starts_with(WS_FRAME_MAGIC));
    let frame = decode_server_binary_frame(&bytes)?;
    assert_eq!(frame.protocol_version, WS_PROTOCOL_VERSION);
    Ok(frame.message)
}

pub(super) fn assert_current_client_frame(bytes: &[u8]) -> anyhow::Result<()> {
    assert!(bytes.starts_with(WS_FRAME_MAGIC));
    let frame = decode_client_binary_frame(bytes)?;
    assert_eq!(frame.protocol_version, WS_PROTOCOL_VERSION);
    Ok(())
}
