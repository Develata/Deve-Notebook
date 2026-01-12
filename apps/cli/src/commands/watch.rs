use std::path::PathBuf;
use std::sync::Arc;
use deve_core::ledger::RepoManager;
use anyhow::Result;

pub fn run(ledger_dir: &PathBuf, vault_path: &PathBuf, snapshot_depth: usize) -> anyhow::Result<()> {
    // Open RepoManager
    let repo = Arc::new(RepoManager::init(ledger_dir, snapshot_depth)?);
    let sync_manager = Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault_path.clone()));
    let watcher = deve_core::watcher::Watcher::new(sync_manager, vault_path.clone());
    println!("Starting watcher on {:?}... Press Ctrl+C to stop.", vault_path);
    watcher.watch()?;
    Ok(())
}
