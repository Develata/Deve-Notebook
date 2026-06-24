// apps\cli\src\commands
//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 11_ui_design/02_desktop#desktop-native-adapter-contract
//!   - 11_ui_design/03_mobile#mobile-native-adapter-contract
//!   - 11_ui_design/01_web#single-binary-distribution
//!   - 14_commands#cli-commands
//!   - 18_release#runtime-observability

use crate::server;
use deve_core::config::{AppProfile, GitBridgeMode, P2pConfig, RuntimeEnvironment, SyncMode};
use deve_core::plugin::runtime::host;
use deve_core::security::AuthConfig;
use reqwest::Client;
use std::path::Path;
use std::sync::Arc;
use tokio::time::{Duration, sleep, timeout};

mod support;
#[cfg(test)]
mod support_tests;
#[cfg(test)]
mod tests;
use support::{
    ensure_native_loopback_default_workspace, find_free_port, init_runtime, load_plugins,
};

pub struct ServeOptions {
    pub port: u16,
    pub snapshot_depth: usize,
    pub dev: bool,
    pub dry_run: bool,
    pub profile: AppProfile,
    pub sync_mode: SyncMode,
    pub git_bridge: GitBridgeMode,
    pub p2p: P2pConfig,
    pub native_loopback: bool,
}

/// 启动后端服务器
///
/// **功能**:
/// 1. 初始化 `RepoManager` (Store B/C Access)
/// 2. 启动 `SyncManager` 进行初始扫描
/// 3. 加载本地插件
/// 4. 启动 WebSocket 服务监听端口
pub async fn run(ledger_dir: &Path, options: ServeOptions) -> anyhow::Result<()> {
    let ServeOptions {
        port,
        snapshot_depth,
        dev,
        dry_run,
        profile,
        sync_mode,
        git_bridge,
        p2p,
        native_loopback,
    } = options;
    let runtime_environment = RuntimeEnvironment::for_serve(dev);
    if dev {
        tracing::warn!("Serve dev mode enabled via --dev");
    }
    if dry_run {
        let _ = init_runtime(ledger_dir, snapshot_depth)?;
        tracing::info!("Serve dry-run OK: repo projection workspaces resolved");
        return Ok(());
    }

    let launch = if native_loopback {
        server::ServerLaunchOptions::native_loopback(port, false)
    } else {
        server::ServerLaunchOptions::release(port)
    }
    .with_runtime_environment(runtime_environment);
    let bind_addr = launch.bind_addr();
    let listener = match tokio::net::TcpListener::bind(bind_addr).await {
        Ok(listener) => listener,
        Err(err) if err.kind() == std::io::ErrorKind::AddrInUse => {
            if native_loopback {
                anyhow::bail!(
                    "Native loopback serve port {} is already in use; refusing proxy fallback",
                    port
                );
            }
            return start_proxy_mode(port, runtime_environment).await;
        }
        Err(err) => return Err(err.into()),
    };

    if native_loopback {
        ensure_native_loopback_default_workspace(ledger_dir, snapshot_depth)?;
    }
    let repo_arc = init_runtime(ledger_dir, snapshot_depth)?;
    let plugins = load_plugins()?;

    if native_loopback {
        tracing::info!("Native loopback serve mode enabled on {}", bind_addr);
    }
    server::start_server_with_bound_listener(
        repo_arc, launch, plugins, profile, sync_mode, git_bridge, p2p, listener,
    )
    .await?;
    Ok(())
}

/// 代理模式: 检测已运行的主进程并以 plugin-host 方式启动
async fn start_proxy_mode(
    port: u16,
    runtime_environment: RuntimeEnvironment,
) -> anyhow::Result<()> {
    let Some(main) = detect_main_node_role(port).await else {
        anyhow::bail!(
            "Serve port {} is already in use, but no healthy Deve main process was detected",
            port
        );
    };
    let main_port = main.port;
    tracing::info!(
        "Main process detected on port {}. Switching to client proxy mode...",
        main_port
    );
    let base_url = format!("http://127.0.0.1:{}", main_port);
    let delegated_secret = AuthConfig::from_env()?.secret;
    let remote = Arc::new(
        crate::server::source_control_proxy::RemoteSourceControlApi::new_with_delegation_secret(
            base_url,
            delegated_secret,
        )?,
    );
    let repo_api: Arc<dyn deve_core::ledger::traits::Repository> = remote.clone();
    host::set_repository(repo_api)?;
    let source_control_api: Arc<dyn deve_core::source_control::DelegatedSourceControlApi> = remote;
    host::set_delegated_source_control_api(source_control_api)?;

    let plugins = load_plugins()?;

    let plugin_port = find_free_port(main_port + 1, 5).unwrap_or(main_port + 1);
    tracing::info!("Plugin host will listen on port {}", plugin_port);
    crate::server::node_role::set_node_role(proxy_node_role(
        plugin_port,
        main_port,
        main.repo_health,
        main.source_control,
        runtime_environment,
    ));
    server::start_plugin_host_only(plugins, plugin_port).await
}

fn proxy_node_role(
    plugin_port: u16,
    main_port: u16,
    repo_health: crate::server::node_role::RepoHealthSummary,
    source_control: crate::server::node_role::SourceControlSummary,
    runtime_environment: RuntimeEnvironment,
) -> crate::server::node_role::NodeRole {
    crate::server::node_role::NodeRole {
        role: "proxy".into(),
        ws_port: plugin_port,
        main_port,
        version: env!("CARGO_PKG_VERSION").into(),
        profile: "proxy".into(),
        delivery: "plugin-host-proxy".into(),
        environment: runtime_environment.as_str().into(),
        repo_health,
        source_control,
        p2p: crate::server::node_role::P2pSummary::disabled(),
        native_service: None,
    }
}

#[cfg(test)]
async fn detect_main_port(port: u16) -> Option<u16> {
    detect_main_node_role(port).await.map(|main| main.port)
}

#[derive(Debug, Clone)]
struct MainNodeRoleProbe {
    port: u16,
    repo_health: crate::server::node_role::RepoHealthSummary,
    source_control: crate::server::node_role::SourceControlSummary,
}

async fn detect_main_node_role(port: u16) -> Option<MainNodeRoleProbe> {
    let mut ports = vec![port];
    for p in port.saturating_sub(2)..=port.saturating_add(4) {
        if !ports.contains(&p) {
            ports.push(p);
        }
    }

    let client = Client::builder().no_proxy().build().ok()?;
    for p in ports {
        if let Some(main) = probe_node_role(&client, p).await {
            return Some(main);
        }
    }
    None
}

async fn probe_node_role(client: &Client, port: u16) -> Option<MainNodeRoleProbe> {
    let url = format!("http://127.0.0.1:{}/api/node/role", port);
    for attempt in 0..3 {
        let req = client.get(&url);
        match timeout(Duration::from_millis(300), req.send()).await {
            Ok(Ok(response)) if response.status().is_success() => {
                if let Some(main) = response
                    .json::<serde_json::Value>()
                    .await
                    .ok()
                    .and_then(|payload| main_node_role_probe(&payload, port))
                {
                    return Some(main);
                }
            }
            _ => {}
        }
        if attempt < 2 {
            sleep(Duration::from_millis(25)).await;
        }
    }
    None
}

fn main_node_role_probe(
    payload: &serde_json::Value,
    probed_port: u16,
) -> Option<MainNodeRoleProbe> {
    if !is_main_node_role_payload(payload, probed_port) {
        return None;
    }
    Some(MainNodeRoleProbe {
        port: probed_port,
        repo_health: repo_health_summary_from_payload(payload),
        source_control: source_control_summary_from_payload(payload),
    })
}

fn repo_health_summary_from_payload(
    payload: &serde_json::Value,
) -> crate::server::node_role::RepoHealthSummary {
    let repo_health = payload.get("repo_health");
    crate::server::node_role::RepoHealthSummary {
        status: repo_health
            .and_then(|repo_health| repo_health.get("status"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .into(),
        local_total: repo_health_usize_field(repo_health, "local_total"),
        healthy: repo_health_usize_field(repo_health, "healthy"),
        degraded: repo_health_usize_field(repo_health, "degraded"),
    }
}

fn repo_health_usize_field(repo_health: Option<&serde_json::Value>, key: &str) -> usize {
    repo_health
        .and_then(|repo_health| repo_health.get(key))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(0)
}

fn source_control_summary_from_payload(
    payload: &serde_json::Value,
) -> crate::server::node_role::SourceControlSummary {
    let git_bridge = payload
        .get("source_control")
        .and_then(|source_control| source_control.get("git_bridge"))
        .and_then(serde_json::Value::as_str)
        .filter(|mode| matches!(*mode, "mirror" | "off" | "unknown"))
        .unwrap_or("unknown");
    crate::server::node_role::SourceControlSummary {
        git_bridge: git_bridge.into(),
    }
}

fn is_main_node_role_payload(payload: &serde_json::Value, probed_port: u16) -> bool {
    let role = payload
        .get("role")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    matches!(role, "main" | "native-main")
        && payload
            .get("ws_port")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|port| port == u64::from(probed_port))
        && payload
            .get("main_port")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|port| port == u64::from(probed_port))
        && has_release_node_role_shape(payload)
}

fn has_release_node_role_shape(payload: &serde_json::Value) -> bool {
    has_str_field(payload, "version")
        && has_str_field(payload, "profile")
        && has_str_field(payload, "delivery")
        && has_str_field(payload, "environment")
        && payload
            .get("repo_health")
            .is_some_and(has_repo_health_shape)
        && payload
            .get("source_control")
            .is_some_and(has_source_control_shape)
}

fn has_repo_health_shape(payload: &serde_json::Value) -> bool {
    has_str_field(payload, "status")
        && has_u64_field(payload, "local_total")
        && has_u64_field(payload, "healthy")
        && has_u64_field(payload, "degraded")
}

fn has_source_control_shape(payload: &serde_json::Value) -> bool {
    payload
        .get("git_bridge")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|mode| matches!(mode, "mirror" | "off" | "unknown"))
}

fn has_str_field(payload: &serde_json::Value, key: &str) -> bool {
    payload
        .get(key)
        .and_then(serde_json::Value::as_str)
        .is_some()
}

fn has_u64_field(payload: &serde_json::Value, key: &str) -> bool {
    payload
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .is_some()
}
