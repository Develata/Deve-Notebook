// apps\cli\src\commands
use crate::server;
use deve_core::config::{AppProfile, SyncMode};
use deve_core::plugin::runtime::host;
use reqwest::Client;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{Duration, timeout};

#[path = "serve_support.rs"]
mod serve_support;
#[cfg(test)]
#[path = "serve_test.rs"]
mod tests;
use serve_support::{find_free_port, init_runtime, load_plugins};

pub struct ServeOptions {
    pub port: u16,
    pub snapshot_depth: usize,
    pub dev: bool,
    pub dry_run: bool,
    pub profile: AppProfile,
    pub sync_mode: SyncMode,
}

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
    options: ServeOptions,
) -> anyhow::Result<()> {
    let ServeOptions {
        port,
        snapshot_depth,
        dev,
        dry_run,
        profile,
        sync_mode,
    } = options;
    if dev {
        if std::env::var("DEVE_ENV").is_err() {
            // 仅对当前 serve 进程显式开启开发模式；不恢复全局隐式 debug 授权。
            unsafe {
                std::env::set_var("DEVE_ENV", "development");
            }
        }
        tracing::warn!("Serve dev mode enabled via --dev");
    }
    if dry_run {
        let _ = init_runtime(ledger_dir, &vault_path, snapshot_depth)?;
        tracing::info!("Serve dry-run OK: {:?}", vault_path);
        return Ok(());
    }

    let bind_addr = format!("0.0.0.0:{}", port);
    if let Err(err) = TcpListener::bind(&bind_addr) {
        if err.kind() == std::io::ErrorKind::AddrInUse {
            return start_proxy_mode(port).await;
        }
        return Err(err.into());
    }

    let (repo_arc, sync_manager) = init_runtime(ledger_dir, &vault_path, snapshot_depth)?;
    sync_manager.scan()?;
    let plugins = load_plugins()?;

    server::start_server(repo_arc, vault_path, port, plugins, profile, sync_mode).await?;
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

async fn detect_main_port(port: u16) -> Option<u16> {
    let mut ports = vec![port];
    for p in port.saturating_sub(2)..=port + 4 {
        if !ports.contains(&p) {
            ports.push(p);
        }
    }

    let client = Client::new();
    for p in ports {
        let url = format!("http://127.0.0.1:{}/api/node/role", p);
        let req = client.get(&url);
        let is_alive = matches!(
            timeout(Duration::from_millis(300), req.send()).await,
            Ok(Ok(_))
        );
        if is_alive {
            return Some(p);
        }
    }
    None
}
