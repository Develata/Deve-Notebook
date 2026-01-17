use std::path::PathBuf;
use std::sync::Arc;
use deve_core::ledger::RepoManager;
use crate::server;

/// 启动后端服务器
/// 
/// **功能**:
/// 1. 初始化 `RepoManager` (Store B/C Access)
/// 2. 启动 `SyncManager` 进行初始扫描
/// 3. 加载本地插件
/// 4. 启动 WebSocket 服务监听端口
pub async fn run(ledger_dir: &PathBuf, vault_path: PathBuf, port: u16, snapshot_depth: usize) -> anyhow::Result<()> {
    // 1. 初始化 RepoManager
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_arc = Arc::new(repo);
    
    // 启动时通过 SyncManager 自动扫描
    let sync_manager = deve_core::sync::SyncManager::new(repo_arc.clone(), vault_path.clone());
    match sync_manager.scan() {
        Ok(_) => {}, // Silent success
        Err(e) => tracing::warn!("启动扫描警告: {:?}", e),
    }
    
    // 2. 加载插件 (Plugins)
    // 默认查找当前目录下的 "plugins" 文件夹
    let plugin_dir = PathBuf::from("plugins");
    let loader = deve_core::plugin::loader::PluginLoader::new(plugin_dir);
    let plugins = match loader.load_all() {
        Ok(p) => {
            tracing::info!("Loaded {} plugins.", p.len());
            p
        },
        Err(e) => {
            tracing::warn!("Failed to load plugins: {}", e);
            vec![]
        }
    };

    server::start_server(repo_arc, vault_path, port, plugins).await?;
    Ok(())
}
