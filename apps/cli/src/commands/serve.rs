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
) -> anyhow::Result<()> {
    let bind_addr = format!("0.0.0.0:{}", port);
    if TcpListener::bind(&bind_addr).is_err() {
        return start_proxy_mode(port).await;
    }

    // 1. 初始化 RepoManager
    let repo = match RepoManager::init(ledger_dir, snapshot_depth, None, None) {
        Ok(r) => r,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("Database already open") {
                return start_proxy_mode(port).await;
            }
            return Err(e);
        }
    };
    let repo_arc = Arc::new(repo);

    // 启动时通过 SyncManager 自动扫描
    let sync_manager = deve_core::sync::SyncManager::new(repo_arc.clone(), vault_path.clone());
    match sync_manager.scan() {
        Ok(_) => {} // Silent success
        Err(e) => tracing::warn!("启动扫描警告: {:?}", e),
    }

    // 2. 加载插件 (Plugins)
    let plugins = load_plugins();

    server::start_server(repo_arc, vault_path, port, plugins).await?;
    Ok(())
}

/// 代理模式: 检测已运行的主进程并以 plugin-host 方式启动
async fn start_proxy_mode(port: u16) -> anyhow::Result<()> {
    let main_port = detect_main_port(port).await;
    tracing::info!(
        "Main process detected on port {}. Switching to client proxy mode...",
        main_port
    );
    let base_url = format!("http://127.0.0.1:{}", main_port);
    let remote =
        Arc::new(crate::server::source_control_proxy::RemoteSourceControlApi::new(base_url));
    let repo_api: Arc<dyn deve_core::ledger::traits::Repository> = remote;
    host::set_repository(repo_api)?;

    let plugins = load_plugins();

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
fn load_plugins() -> Vec<Box<dyn deve_core::plugin::runtime::PluginRuntime>> {
    let plugin_dir = PathBuf::from("plugins");
    let loader = PluginLoader::new(plugin_dir);
    match loader.load_all() {
        Ok(p) => {
            tracing::info!("Loaded {} plugins.", p.len());
            p
        }
        Err(e) => {
            tracing::warn!("Failed to load plugins: {}", e);
            vec![]
        }
    }
}

async fn detect_main_port(port: u16) -> u16 {
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
            return p;
        }
    }
    port
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
