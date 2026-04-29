use super::{ServeOptions, detect_main_port, run};
use axum::{Router, routing::get};
use deve_core::config::{AppProfile, SyncMode};
use std::net::{SocketAddr, TcpListener};
use tempfile::TempDir;

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("read local addr")
        .port()
}

async fn spawn_status_server(status: axum::http::StatusCode) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let addr = listener.local_addr().expect("server addr");
    let app = Router::new().route("/api/node/role", get(move || async move { status }));
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve test app");
    });
    addr
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_returns_none_without_healthy_server() {
    let port = free_port();
    assert_eq!(detect_main_port(port).await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_finds_deve_process_via_node_role() {
    let addr = spawn_status_server(axum::http::StatusCode::OK).await;
    assert_eq!(detect_main_port(addr.port()).await, Some(addr.port()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_accepts_non_success_status() {
    let addr = spawn_status_server(axum::http::StatusCode::UNAUTHORIZED).await;
    assert_eq!(detect_main_port(addr.port()).await, Some(addr.port()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serve_dry_run_validates_runtime_without_binding() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let vault_dir = dir.path().join("vault");
    std::fs::create_dir_all(&vault_dir).expect("create vault");

    run(
        &ledger_dir,
        vault_dir,
        ServeOptions {
            port: free_port(),
            snapshot_depth: 8,
            dev: false,
            dry_run: true,
            profile: AppProfile::Standard,
            sync_mode: SyncMode::Auto,
            native_loopback: false,
        },
    )
    .await
    .expect("serve dry-run");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_loopback_refuses_proxy_fallback_when_port_is_occupied() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let vault_dir = dir.path().join("vault");
    std::fs::create_dir_all(&vault_dir).expect("create vault");
    let listener = TcpListener::bind("127.0.0.1:0").expect("occupy loopback port");
    let port = listener.local_addr().expect("listener addr").port();

    let err = run(
        &ledger_dir,
        vault_dir,
        ServeOptions {
            port,
            snapshot_depth: 8,
            dev: false,
            dry_run: false,
            profile: AppProfile::Standard,
            sync_mode: SyncMode::Auto,
            native_loopback: true,
        },
    )
    .await
    .expect_err("native loopback must fail closed on occupied port");

    assert!(err.to_string().contains("refusing proxy fallback"), "{err}");
}
