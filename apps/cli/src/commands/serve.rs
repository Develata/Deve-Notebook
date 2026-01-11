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
    
    server::start_server(ledger_arc, vault_path, port).await?;
    Ok(())
}
