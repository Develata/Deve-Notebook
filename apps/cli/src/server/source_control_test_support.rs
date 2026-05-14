use super::source_control_proxy::RemoteSourceControlApi;
use super::{AppState, router, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::security::AuthConfig;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;

pub(super) struct ProxyHarness {
    pub dir: TempDir,
    pub repo: Arc<RepoManager>,
    pub sync_manager: Arc<deve_core::sync::SyncManager>,
    pub base_url: String,
    pub client: reqwest::Client,
    pub proxy: RemoteSourceControlApi,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl ProxyHarness {
    pub(super) async fn spawn() -> anyhow::Result<Self> {
        let dir = tempdir()?;
        let vault = dir.path().join("vault");
        let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
        repo.set_vault_root(&vault);
        let repo = Arc::new(repo);
        let (tx, _rx) = broadcast::channel(16);
        let sync_manager = Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault.clone()));
        let state = Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: sync_manager.clone(),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo.clone(),
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key: security::load_or_generate_identity_key(&dir.path().join("host"))?,
        });
        let mut auth_config = AuthConfig::dev_default()?;
        auth_config.allow_anonymous_localhost = true;
        let app = router::build_app(state, 3001, Arc::new(auth_config))?
            .into_make_service_with_connect_info::<std::net::SocketAddr>();
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let base_url = format!("http://{}", listener.local_addr()?);
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .expect("serve app");
        });
        Ok(Self {
            dir,
            repo,
            sync_manager,
            base_url: base_url.clone(),
            client: local_client(),
            proxy: RemoteSourceControlApi::new(base_url)?,
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

fn local_client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("build local test HTTP client")
}
