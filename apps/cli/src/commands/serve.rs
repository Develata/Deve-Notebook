use std::path::PathBuf;
use std::sync::Arc;
use deve_core::ledger::Ledger;
use crate::server;

pub async fn run(ledger_path: &PathBuf, vault_path: PathBuf, port: u16, snapshot_depth: usize) -> anyhow::Result<()> {
    // 1. Initialize Ledger
    let ledger = Ledger::init(ledger_path, snapshot_depth)?;
    let ledger_arc = Arc::new(ledger);
    
    // Auto-scan on startup via SyncManager
    let sync_manager = deve_core::sync::SyncManager::new(ledger_arc.clone(), vault_path.clone());
    match sync_manager.scan() {
        Ok(_) => {}, // Silent success
        Err(e) => tracing::warn!("启动扫描警告: {:?}", e),
    }
    
    // 2. Load Plugins
    // We look for a 'plugins' directory in the current working directory or adjacent to the vault.
    // For now, let's check a "plugins" folder in the current directory.
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

    server::start_server(ledger_arc, vault_path, port, plugins).await?;
    Ok(())
}
