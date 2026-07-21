//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 03_storage/watcher#watcher-contract
//!   - 15_settings#configuration-settings

use deve_cli::server::{ServerLaunchOptions, start_server_with_bound_listener};
use deve_core::config::{AppProfile, P2pConfig, RuntimeEnvironment, SyncMode};
use deve_core::ledger::RepoManager;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn zero_repo_server_composition_accepts_connections_without_creating_authority()
-> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = Arc::new(RepoManager::init_empty_host(&ledger, 8)?);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let launch = ServerLaunchOptions::loopback_release(port)
        .with_runtime_environment(RuntimeEnvironment::Development)
        .with_repo_creation_projection_base(Some(dir.path().join("notes")));

    let server = tokio::spawn(start_server_with_bound_listener(
        repo,
        launch,
        Vec::new(),
        AppProfile::Standard,
        SyncMode::Auto,
        P2pConfig::default(),
        listener,
    ));

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;

    assert!(
        std::fs::read_dir(ledger.join("local"))?.all(|entry| entry
            .map(|entry| entry.path().extension().is_none_or(|ext| ext != "redb"))
            .unwrap_or(false)),
        "zero-repo server startup must not create a local authority database"
    );
    server.abort();
    let _ = server.await;
    Ok(())
}
