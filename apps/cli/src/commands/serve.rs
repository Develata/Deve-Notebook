// apps\cli\src\commands
use crate::server;
use deve_core::ledger::RepoManager;
use deve_core::plugin::loader::PluginLoader;
use deve_core::plugin::runtime::host;
use reqwest::Client;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{Duration, timeout};

/// 启动后端服务器
///
/// **功能**:
/// 1. 初始化 `RepoManager` (Store B/C Access)
/// 2. 启动 `SyncManager` 进行初始扫描
/// 3. 加载本地插件
/// 4. 启动 WebSocket 服务监听端口
pub async fn run(
    ledger_dir: &PathBuf,
    vault_path: PathBuf,
    port: u16,
    snapshot_depth: usize,
    dev: bool,
) -> anyhow::Result<()> {
    if dev {
        if std::env::var("DEVE_ENV").is_err() {
            // 仅对当前 serve 进程显式开启开发模式；不恢复全局隐式 debug 授权。
            unsafe {
                std::env::set_var("DEVE_ENV", "development");
            }
        }
        tracing::warn!("Serve dev mode enabled via --dev");
    }

    let bind_addr = format!("0.0.0.0:{}", port);
    if let Err(err) = TcpListener::bind(&bind_addr) {
        if err.kind() == std::io::ErrorKind::AddrInUse {
            return start_proxy_mode(port).await;
        }
        return Err(err.into());
    }

    // 1. 初始化 RepoManager
    let mut repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    repo.set_vault_root_checked(&vault_path)?;
    let repo_arc = Arc::new(repo);

    // 启动时通过 SyncManager 自动扫描
    let sync_manager = deve_core::sync::SyncManager::new(repo_arc.clone(), vault_path.clone());
    sync_manager.scan()?;

    // 2. 加载插件 (Plugins)
    let plugins = load_plugins()?;

    server::start_server(repo_arc, vault_path, port, plugins).await?;
    Ok(())
}

/// 代理模式: 检测已运行的主进程并以 plugin-host 方式启动
async fn start_proxy_mode(port: u16) -> anyhow::Result<()> {
    let Some(main_port) = detect_main_port(port).await else {
        anyhow::bail!(
            "Serve port {} is already in use, but no healthy Deve main process was detected",
            port
        );
    };
    tracing::info!(
        "Main process detected on port {}. Switching to client proxy mode...",
        main_port
    );
    let base_url = format!("http://127.0.0.1:{}", main_port);
    let remote =
        Arc::new(crate::server::source_control_proxy::RemoteSourceControlApi::new(base_url));
    let repo_api: Arc<dyn deve_core::ledger::traits::Repository> = remote;
    host::set_repository(repo_api)?;

    let plugins = load_plugins()?;

    let plugin_port = find_free_port(main_port + 1, 5).unwrap_or(main_port + 1);
    tracing::info!("Plugin host will listen on port {}", plugin_port);
    crate::server::node_role::set_node_role(crate::server::node_role::NodeRole {
        role: "proxy".into(),
        ws_port: plugin_port,
        main_port,
    });
    server::start_plugin_host_only(plugins, plugin_port).await
}

/// 加载 `plugins/` 目录下的所有 Rhai 插件
fn load_plugins() -> anyhow::Result<Vec<Box<dyn deve_core::plugin::runtime::PluginRuntime>>> {
    let plugin_dir = PathBuf::from("plugins");
    let loader = PluginLoader::new(plugin_dir);
    let plugins = loader.load_all_strict()?;
    tracing::info!("Loaded {} plugins.", plugins.len());
    Ok(plugins)
}

async fn detect_main_port(port: u16) -> Option<u16> {
    let mut ports = vec![port];
    for p in port.saturating_sub(2)..=port + 4 {
        if !ports.contains(&p) {
            ports.push(p);
        }
    }

    let client = Client::new();
    for p in ports {
        let url = format!("http://127.0.0.1:{}/api/repo/docs", p);
        let req = client.get(&url);
        let is_ok = matches!(
            timeout(Duration::from_millis(300), req.send()).await,
            Ok(Ok(resp)) if resp.status().is_success()
        );
        if is_ok {
            return Some(p);
        }
    }
    None
}

fn find_free_port(start: u16, span: u16) -> Option<u16> {
    for p in start..=start.saturating_add(span) {
        let addr = format!("0.0.0.0:{}", p);
        if TcpListener::bind(&addr).is_ok() {
            return Some(p);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::detect_main_port;
    use axum::{Router, routing::get};
    use std::net::{SocketAddr, TcpListener};

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
        let app = Router::new().route("/api/repo/docs", get(move || async move { status }));
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
    async fn detect_main_port_finds_healthy_repo_docs_endpoint() {
        let addr = spawn_status_server(axum::http::StatusCode::OK).await;
        assert_eq!(detect_main_port(addr.port()).await, Some(addr.port()));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn detect_main_port_ignores_unhealthy_repo_docs_endpoint() {
        let addr = spawn_status_server(axum::http::StatusCode::SERVICE_UNAVAILABLE).await;
        assert_eq!(detect_main_port(addr.port()).await, None);
    }
}
