//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! End-to-end WebSocket protocol frame acceptance coverage.

use super::{router, sync_hello_test_support::build_state};
use deve_core::protocol::frame::{
    WS_FRAME_MAGIC, WS_PROTOCOL_VERSION, decode_client_binary_frame, decode_server_binary_frame,
    encode_client_binary, encode_client_binary_with_version,
};
use deve_core::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use deve_core::security::AuthConfig;
use futures::{SinkExt, Stream, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

struct WsHarness {
    _dir: TempDir,
    ws_url: String,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl WsHarness {
    async fn spawn() -> anyhow::Result<Self> {
        let (dir, state, _repo_id) = build_state()?;
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
            ws_url: format!("ws://{addr}/ws"),
            shutdown: Some(shutdown_tx),
            task,
        })
    }

    async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.task.await;
    }
}

async fn recv_server_message<S>(ws: &mut S) -> anyhow::Result<ServerMessage>
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

fn assert_current_client_frame(bytes: &[u8]) -> anyhow::Result<()> {
    assert!(bytes.starts_with(WS_FRAME_MAGIC));
    let frame = decode_client_binary_frame(bytes)?;
    assert_eq!(frame.protocol_version, WS_PROTOCOL_VERSION);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_roundtrips_versioned_binary_ping() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let (mut ws, _response) = connect_async(&harness.ws_url).await?;
    let bytes = encode_client_binary(&ClientMessage::Ping)?;
    assert_current_client_frame(&bytes)?;

    ws.send(Message::Binary(bytes)).await?;

    assert!(matches!(
        recv_server_message(&mut ws).await?,
        ServerMessage::Pong
    ));
    harness.shutdown().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_rejects_unsupported_protocol_version() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let (mut ws, _response) = connect_async(&harness.ws_url).await?;
    let unsupported_version = WS_PROTOCOL_VERSION + 1;
    let bytes = encode_client_binary_with_version(&ClientMessage::Ping, unsupported_version)?;

    ws.send(Message::Binary(bytes)).await?;

    match recv_server_message(&mut ws).await? {
        ServerMessage::ProtocolError { error, .. } => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("unsupported WS protocol version"))
            );
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
    harness.shutdown().await;
    Ok(())
}
