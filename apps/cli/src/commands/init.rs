use std::path::PathBuf;
use deve_core::ledger::Ledger;

pub fn run(ledger_path: &PathBuf, vault_path: &PathBuf, path: PathBuf, snapshot_depth: usize) -> anyhow::Result<()> {
    println!("Initializing ledger at {:?}...", ledger_path);
    // 1. Initialize Ledger (creates file)
    let _ = Ledger::init(ledger_path, snapshot_depth)?;
    std::fs::create_dir_all(vault_path)?;
    println!("Initialization complete.");
    Ok(())
}
