use std::path::PathBuf;
use deve_core::ledger::RepoManager;

pub fn run(ledger_dir: &PathBuf, vault_path: &PathBuf, path: PathBuf, snapshot_depth: usize) -> anyhow::Result<()> {
    println!("Initializing ledger at {:?}...", ledger_dir);
    // 1. Initialize RepoManager (creates directory structure)
    let _ = RepoManager::init(ledger_dir, snapshot_depth)?;
    std::fs::create_dir_all(vault_path)?;
    println!("Initialization complete.");
    Ok(())
}
