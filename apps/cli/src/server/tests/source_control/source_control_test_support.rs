use super::source_control_proxy::RemoteSourceControlApi;
use super::{
    AppState, auth, router, security, source_control_grants::AuthSessionId,
    tree_state::RepoTreeRegistry,
};
use deve_core::config::{GitBridgeMode, SyncMode};
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
    pub state: Arc<AppState>,
    pub auth_session_id: AuthSessionId,
    pub base_url: String,
    pub client: reqwest::Client,
    pub proxy: RemoteSourceControlApi,
    dev_session_secret: String,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl ProxyHarness {
    pub(super) async fn spawn() -> anyhow::Result<Self> {
        Self::spawn_with_git_bridge(GitBridgeMode::Mirror).await
    }

    pub(super) async fn spawn_with_git_bridge(git_bridge: GitBridgeMode) -> anyhow::Result<Self> {
        let dir = tempdir()?;
        let projection_base = dir.path().join("notes");
        let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
        repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
        let repo = Arc::new(repo);
        let (tx, _rx) = broadcast::channel(16);
        let sync_manager = Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?);
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
            git_bridge,
        });
        let mut auth_config = AuthConfig::dev_default()?;
        auth_config.allow_anonymous_localhost = true;
        let dev_session_secret = auth_config.secret.clone();
        let dev_session_cookie_header =
            auth::dev_session::cookie_header_for_test(&dev_session_secret, "source-control-harness");
        let auth_session_id = AuthSessionId::from_dev_session_cookie(
            &auth_config.username,
            auth_config.token_version,
            "source-control-harness",
        );
        let app = router::build_app(state.clone(), 3001, Arc::new(auth_config))?
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
            state,
            auth_session_id,
            base_url: base_url.clone(),
            client: local_client(&dev_session_cookie_header),
            proxy: RemoteSourceControlApi::new(base_url)?,
            dev_session_secret,
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

    pub(super) fn grant_browser_write(&self, scope_nonce: u64) -> anyhow::Result<()> {
        let repo_name = self.repo.local_repo_name();
        let repo_id = self
            .repo
            .get_repo_info_for(None, Some(repo_name))?
            .ok_or_else(|| anyhow::anyhow!("missing local repo info"))?
            .uuid;
        self.state.source_control_write_grants().grant(
            self.auth_session_id.clone(),
            repo_id,
            PeerId::new("test-peer"),
            scope_nonce,
        );
        Ok(())
    }

    pub(super) fn dev_session_cookie_header_for(&self, nonce: &str) -> String {
        auth::dev_session::cookie_header_for_test(&self.dev_session_secret, nonce)
    }
}

fn local_client(cookie_header: &str) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::COOKIE,
        reqwest::header::HeaderValue::from_str(cookie_header)
            .expect("test dev session cookie header"),
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .no_proxy()
        .build()
        .expect("build local test HTTP client")
}
