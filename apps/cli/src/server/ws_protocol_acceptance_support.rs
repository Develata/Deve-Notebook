//! plan_ref:
//!   - 05_network#server-ws-runtime
//!
//! Shared end-to-end WebSocket protocol acceptance harness.

use super::{router, sync_hello_test_support::build_state};
use deve_core::models::PeerId;
use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::protocol::frame::{
    WS_FRAME_MAGIC, WS_PROTOCOL_VERSION, decode_client_binary_frame, decode_server_binary_frame,
    encode_client_binary,
};
use deve_core::security::{AuthConfig, IdentityKeyPair};
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{Duration, timeout};
use tokio_tungstenite::tungstenite::{Error as WsError, Message};
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

pub(super) async fn switch_to_notes_repo<S>(
    ws: &mut S,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
) -> anyhow::Result<()>
where
    S: Sink<Message, Error = WsError> + Stream<Item = Result<Message, WsError>> + Unpin,
{
    send_client_message(
        ws,
        ClientMessage::SwitchRepoExact {
            name: "notes".into(),
            repo_id,
            switch_nonce: Some(scope_nonce),
        },
    )
    .await?;
    assert_repo_switched(recv_server_message(ws).await?, repo_id, scope_nonce);
    assert_doc_list(recv_server_message(ws).await?, repo_id, scope_nonce);
    assert_tree_update(recv_server_message(ws).await?, repo_id, scope_nonce);
    Ok(())
}

pub(super) async fn expect_sync_hello_and_shadow_list<S>(
    ws: &mut S,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
    local_peer: &PeerId,
    remote: &IdentityKeyPair,
) -> anyhow::Result<()>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
{
    assert_sync_hello(recv_server_message(ws).await?, repo_id, scope_nonce, local_peer);
    assert_shadow_list(recv_server_message(ws).await?, scope_nonce, remote);
    Ok(())
}

pub(super) async fn send_client_message<S>(ws: &mut S, msg: ClientMessage) -> anyhow::Result<()>
where
    S: Sink<Message, Error = WsError> + Unpin,
{
    let bytes = encode_client_binary(&msg)?;
    assert_current_client_frame(&bytes)?;
    ws.send(Message::Binary(bytes)).await?;
    Ok(())
}

pub(super) async fn recv_server_message<S>(ws: &mut S) -> anyhow::Result<ServerMessage>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
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

pub(super) async fn recv_optional_server_message<S>(
    ws: &mut S,
    quiet_for: Duration,
) -> anyhow::Result<Option<ServerMessage>>
where
    S: Stream<Item = Result<Message, WsError>> + Unpin,
{
    match timeout(quiet_for, recv_server_message(ws)).await {
        Ok(result) => result.map(Some),
        Err(_) => Ok(None),
    }
}

pub(super) fn assert_current_client_frame(bytes: &[u8]) -> anyhow::Result<()> {
    assert!(bytes.starts_with(WS_FRAME_MAGIC));
    let frame = decode_client_binary_frame(bytes)?;
    assert_eq!(frame.protocol_version, WS_PROTOCOL_VERSION);
    Ok(())
}

fn assert_repo_switched(message: ServerMessage, repo_id: uuid::Uuid, scope_nonce: u64) {
    match message {
        ServerMessage::RepoSwitched {
            branch,
            name,
            uuid,
            switch_nonce,
        } => {
            assert_eq!(branch, None);
            assert_eq!(name, "notes");
            assert_eq!(uuid, repo_id.to_string());
            assert_eq!(switch_nonce, Some(scope_nonce));
        }
        other => panic!("expected RepoSwitched, got {other:?}"),
    }
}

fn assert_doc_list(message: ServerMessage, repo_id: uuid::Uuid, scope_nonce: u64) {
    match message {
        ServerMessage::DocList {
            repo_id: Some(actual),
            branch,
            scope_nonce: actual_scope,
            ..
        } => {
            assert_eq!(actual, repo_id);
            assert_eq!(branch, None);
            assert_eq!(actual_scope, Some(scope_nonce));
        }
        other => panic!("expected DocList, got {other:?}"),
    }
}

fn assert_tree_update(message: ServerMessage, repo_id: uuid::Uuid, scope_nonce: u64) {
    match message {
        ServerMessage::TreeUpdate {
            repo_id: Some(actual),
            branch,
            scope_nonce: actual_scope,
            ..
        } => {
            assert_eq!(actual, repo_id);
            assert_eq!(branch, None);
            assert_eq!(actual_scope, Some(scope_nonce));
        }
        other => panic!("expected TreeUpdate, got {other:?}"),
    }
}

fn assert_sync_hello(
    message: ServerMessage,
    repo_id: uuid::Uuid,
    scope_nonce: u64,
    local_peer: &PeerId,
) {
    match message {
        ServerMessage::SyncHello {
            peer_id,
            repo_id: actual,
            scope_nonce: actual_scope,
            pub_key,
            signature,
            ..
        } => {
            assert_eq!(&peer_id, local_peer);
            assert_eq!(actual, repo_id);
            assert_eq!(actual_scope, scope_nonce);
            assert!(!pub_key.is_empty());
            assert!(!signature.is_empty());
        }
        other => panic!("expected SyncHello, got {other:?}"),
    }
}

fn assert_shadow_list(message: ServerMessage, scope_nonce: u64, remote: &IdentityKeyPair) {
    match message {
        ServerMessage::ShadowList {
            scope_nonce: actual_scope,
            shadows,
            ..
        } => {
            assert_eq!(actual_scope, Some(scope_nonce));
            assert!(!shadows.contains(&remote.peer_id().to_string()));
        }
        other => panic!("expected ShadowList, got {other:?}"),
    }
}
