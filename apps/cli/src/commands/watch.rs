use std::path::PathBuf;
use std::sync::Arc;
use deve_core::ledger::Ledger;
use anyhow::Result;

pub fn run(ledger_path: &PathBuf, vault_path: &PathBuf, snapshot_depth: usize) -> anyhow::Result<()> {
    // Open Ledger
    let ledger = Arc::new(Ledger::init(ledger_path, snapshot_depth)?);
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new(ledger.clone(), vault_path.clone()));
    let watcher = deve_core::watcher::Watcher::new(sync_manager, vault_path.clone());
    println!("Starting watcher on {:?}... Press Ctrl+C to stop.", vault_path);
    watcher.watch()?;
    Ok(())
}
