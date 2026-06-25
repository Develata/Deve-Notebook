use super::{
    ServeOptions, detect_main_node_role, detect_main_port, proxy_auth_config, proxy_node_role, run,
};
use axum::{Json, Router, routing::get};
use deve_core::config::{AppProfile, GitBridgeMode, P2pConfig, RuntimeEnvironment, SyncMode};
use std::ffi::OsString;
use std::net::{SocketAddr, TcpListener};
use std::sync::Mutex;
use tempfile::TempDir;

static ENV_LOCK: Mutex<()> = Mutex::new(());

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

async fn spawn_json_server(payload: serde_json::Value) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let addr = listener.local_addr().expect("server addr");
    let app = Router::new().route(
        "/api/node/role",
        get(move || {
            let payload = payload.clone();
            async move { Json(payload) }
        }),
    );
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve test app");
    });
    addr
}

async fn spawn_node_role_server(role: &'static str) -> SocketAddr {
    spawn_node_role_server_with_git_bridge(role, "mirror").await
}

async fn spawn_node_role_server_with_git_bridge(
    role: &'static str,
    git_bridge: &'static str,
) -> SocketAddr {
    spawn_node_role_server_with_git_bridge_and_repo_health(role, git_bridge, "healthy", 1, 1, 0)
        .await
}

async fn spawn_node_role_server_with_git_bridge_and_repo_health(
    role: &'static str,
    git_bridge: &'static str,
    repo_health_status: &'static str,
    local_total: u64,
    healthy: u64,
    degraded: u64,
) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let addr = listener.local_addr().expect("server addr");
    let port = addr.port();
    let app = Router::new().route(
        "/api/node/role",
        get(move || async move {
            Json(node_role_payload_with_repo_health(
                role,
                port,
                git_bridge,
                repo_health_status,
                local_total,
                healthy,
                degraded,
            ))
        }),
    );
    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve test app");
    });
    addr
}

fn node_role_payload_with_repo_health(
    role: &str,
    port: u16,
    git_bridge: &str,
    repo_health_status: &str,
    local_total: u64,
    healthy: u64,
    degraded: u64,
) -> serde_json::Value {
    serde_json::json!({
        "role": role,
        "ws_port": port,
        "main_port": port,
        "version": "0.0.1",
        "profile": "standard",
        "delivery": "embedded-frontend",
        "environment": "development",
        "repo_health": {
            "status": repo_health_status,
            "local_total": local_total,
            "healthy": healthy,
            "degraded": degraded,
        },
        "source_control": {
            "git_bridge": git_bridge,
        },
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_returns_none_without_healthy_server() {
    let port = free_port();
    assert_eq!(detect_main_port(port).await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_finds_deve_process_via_node_role() {
    let addr = spawn_node_role_server("main").await;
    assert_eq!(detect_main_port(addr.port()).await, Some(addr.port()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_accepts_native_main_node_role() {
    let addr = spawn_node_role_server("native-main").await;
    assert_eq!(detect_main_port(addr.port()).await, Some(addr.port()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_node_role_preserves_source_control_git_bridge_mode() {
    let addr = spawn_node_role_server_with_git_bridge("main", "off").await;
    let detected = detect_main_node_role(addr.port())
        .await
        .expect("main node role");

    assert_eq!(detected.port, addr.port());
    assert_eq!(detected.source_control.git_bridge, "off");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_node_role_preserves_repo_health_summary() {
    let addr = spawn_node_role_server_with_git_bridge_and_repo_health(
        "main", "mirror", "degraded", 3, 2, 1,
    )
    .await;

    let detected = detect_main_node_role(addr.port())
        .await
        .expect("main node role");

    assert_eq!(detected.repo_health.status, "degraded");
    assert_eq!(detected.repo_health.local_total, 3);
    assert_eq!(detected.repo_health.healthy, 2);
    assert_eq!(detected.repo_health.degraded, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_rejects_non_success_status() {
    let addr = spawn_status_server(axum::http::StatusCode::UNAUTHORIZED).await;
    assert_eq!(detect_main_port(addr.port()).await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_rejects_non_node_role_payload() {
    let addr = spawn_json_server(serde_json::json!({
        "status": "ok",
    }))
    .await;

    assert_eq!(detect_main_port(addr.port()).await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_rejects_partial_node_role_payload() {
    let addr = spawn_json_server(serde_json::json!({
        "role": "main",
        "ws_port": 3001,
        "main_port": 3001,
    }))
    .await;

    assert_eq!(detect_main_port(addr.port()).await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_rejects_proxy_node_role_payload() {
    let addr = spawn_json_server(serde_json::json!({
        "role": "proxy",
        "ws_port": 3002,
        "main_port": 3001,
    }))
    .await;

    assert_eq!(detect_main_port(addr.port()).await, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detect_main_port_rejects_node_role_for_different_port() {
    let addr = spawn_json_server(serde_json::json!({
        "role": "main",
        "ws_port": 3001,
        "main_port": 3001,
    }))
    .await;

    assert_eq!(detect_main_port(addr.port()).await, None);
}

#[test]
fn proxy_node_role_uses_delegated_git_bridge_mode() {
    let role = proxy_node_role(
        3002,
        3001,
        crate::server::node_role::RepoHealthSummary::from_degraded_count(2, 1),
        crate::server::node_role::SourceControlSummary {
            git_bridge: "off".into(),
        },
        RuntimeEnvironment::Development,
    );

    assert_eq!(role.role, "proxy");
    assert_eq!(role.ws_port, 3002);
    assert_eq!(role.main_port, 3001);
    assert_eq!(role.delivery, "plugin-host-proxy");
    assert_eq!(role.environment, "development");
    assert_eq!(role.source_control.git_bridge, "off");
    assert_eq!(role.repo_health.status, "degraded");
    assert_eq!(role.repo_health.local_total, 2);
    assert_eq!(role.repo_health.degraded, 1);
}

#[test]
fn proxy_auth_config_uses_serve_dev_runtime_environment() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _secret = EnvGuard::set("AUTH_SECRET", None);
    let _pass = EnvGuard::set("AUTH_PASS", None);
    let _deve_env = EnvGuard::set("DEVE_ENV", Some("production"));

    let auth = proxy_auth_config(RuntimeEnvironment::Development)
        .expect("proxy dev mode should use development auth defaults");

    assert_eq!(auth.secret, "deve_dev_secret_key_32bytes_ok!!");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serve_dry_run_validates_runtime_without_binding() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let repo = deve_core::ledger::RepoManager::init(&ledger_dir, 8, Some("default"), None)
        .expect("init repo");
    repo.set_projection_base_for_local_repo("default", &projection_base)
        .expect("locator");

    run(
        &ledger_dir,
        ServeOptions {
            port: free_port(),
            snapshot_depth: 8,
            dev: false,
            dry_run: true,
            profile: AppProfile::Standard,
            sync_mode: SyncMode::Auto,
            git_bridge: GitBridgeMode::Mirror,
            p2p: P2pConfig::default(),
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
    let listener = TcpListener::bind("127.0.0.1:0").expect("occupy loopback port");
    let port = listener.local_addr().expect("listener addr").port();

    let err = run(
        &ledger_dir,
        ServeOptions {
            port,
            snapshot_depth: 8,
            dev: false,
            dry_run: false,
            profile: AppProfile::Standard,
            sync_mode: SyncMode::Auto,
            git_bridge: GitBridgeMode::Mirror,
            p2p: P2pConfig::default(),
            native_loopback: true,
        },
    )
    .await
    .expect_err("native loopback must fail closed on occupied port");

    assert!(err.to_string().contains("refusing proxy fallback"), "{err}");
}

// ENV_LOCK serializes tests that mutate process-wide env; it must stay held across
// the run().await below, so the await-holding-lock lint does not apply here.
#[allow(clippy::await_holding_lock)]
#[tokio::test(flavor = "current_thread")]
async fn serve_dev_does_not_mutate_existing_deve_env() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let _env = EnvGuard::set("DEVE_ENV", Some("production"));
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let repo = deve_core::ledger::RepoManager::init(&ledger_dir, 8, Some("default"), None)
        .expect("init repo");
    repo.set_projection_base_for_local_repo("default", &projection_base)
        .expect("locator");

    run(
        &ledger_dir,
        ServeOptions {
            port: free_port(),
            snapshot_depth: 8,
            dev: true,
            dry_run: true,
            profile: AppProfile::Standard,
            sync_mode: SyncMode::Auto,
            git_bridge: GitBridgeMode::Mirror,
            p2p: P2pConfig::default(),
            native_loopback: false,
        },
    )
    .await
    .expect("serve dev dry-run");

    assert_eq!(std::env::var("DEVE_ENV").as_deref(), Ok("production"));
}

struct EnvGuard {
    key: &'static str,
    previous: Option<OsString>,
}

impl EnvGuard {
    fn set(key: &'static str, value: Option<&str>) -> Self {
        let previous = std::env::var_os(key);
        unsafe {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
        Self { key, previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            if let Some(previous) = self.previous.as_ref() {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}
