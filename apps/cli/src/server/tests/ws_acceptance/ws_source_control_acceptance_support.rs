//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 05_diff_logic#source-control-runtime

use super::{router, sync_hello_test_support::build_state};
use crate::server::AppState;
use deve_core::models::PeerId;
use deve_core::protocol::ClientMessage;
use deve_core::security::AuthConfig;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

pub(super) type TestWs = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(super) struct SourceControlWsHarness {
    _dir: TempDir,
    state: Arc<AppState>,
    pub(super) repo_id: uuid::Uuid,
    pub(super) local_peer_id: PeerId,
    ws_url: String,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl SourceControlWsHarness {
    pub(super) async fn spawn() -> anyhow::Result<Self> {
        let (dir, state, repo_id) = build_state()?;
        let local_peer_id = state.identity_key.peer_id();
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let mut auth_config = AuthConfig::dev_default()?;
        auth_config.allow_anonymous_localhost = true;
        let app = router::build_app(state.clone(), addr.port(), Arc::new(auth_config))?
            .into_make_service_with_connect_info::<SocketAddr>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("serve ws source-control harness");
        });
        Ok(Self {
            _dir: dir,
            state,
            repo_id,
            local_peer_id,
            ws_url: format!("ws://{addr}/ws"),
            shutdown: Some(shutdown_tx),
            task,
        })
    }

    pub(super) async fn connect(&self) -> anyhow::Result<TestWs> {
        let (ws, _response) = connect_async(&self.ws_url).await?;
        Ok(ws)
    }

    pub(super) fn seed_pending_added(&self, path: &str, content: &str) -> anyhow::Result<()> {
        self.write_workspace_file(path, content)?;
        self.state
            .repo
            .run_on_local_repo(self.state.repo.local_repo_name(), |db| {
                pending_fs::upsert(
                    db,
                    &PendingFsEntry {
                        path: path.into(),
                        renamed_from: None,
                        doc_id: None,
                        change_type: ChangeStatus::Added,
                        content_hash: pending_fs::content_hash(content),
                        detected_at: 1,
                        has_conflict: false,                    },
                )
            })
    }

    pub(super) async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.task.await;
    }

    fn write_workspace_file(&self, path: &str, content: &str) -> anyhow::Result<()> {
        let repo_name = self.state.repo.local_repo_name();
        self.state
            .repo
            .ensure_local_repo_workspace_identity(repo_name)?;
        let abs = self
            .state
            .repo
            .local_repo_workspace_path(repo_name, path)?;
        if let Some(parent) = abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(abs, content)?;
        Ok(())
    }
}

pub(super) async fn send_scoped(
    ws: &mut TestWs,
    msg: impl FnOnce(Option<u64>) -> ClientMessage,
    scope_nonce: u64,
) -> anyhow::Result<()> {
    super::ws_protocol_acceptance_support::send_client_message(ws, msg(Some(scope_nonce))).await
}
