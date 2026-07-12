use super::build_sync_engine;
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use std::sync::Arc;

#[test]
fn server_sync_engine_uses_configured_sync_mode() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 8, Some("notes"), Some("urn:test:notes"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;

    let engine = build_sync_engine(PeerId::new("local"), Arc::new(repo), SyncMode::Manual);

    assert_eq!(engine.sync_mode(), SyncMode::Manual);
    Ok(())
}
use super::serve_router_until_shutdown;
use axum::{Router, routing::get};
use tokio::sync::oneshot;

#[tokio::test]
async fn native_loopback_graceful_shutdown_stops_bound_server() {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("listener");
    let addr = listener.local_addr().expect("addr");
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let task = tokio::spawn(serve_router_until_shutdown(
        listener,
        Router::new().route("/health", get(|| async { "ok" })),
        async move {
            let _ = shutdown_rx.await;
        },
    ));

    tokio::net::TcpStream::connect(addr)
        .await
        .expect("server accepts connections");
    shutdown_tx.send(()).expect("signal shutdown");
    tokio::time::timeout(std::time::Duration::from_secs(1), task)
        .await
        .expect("bounded shutdown")
        .expect("join")
        .expect("server result");
}
