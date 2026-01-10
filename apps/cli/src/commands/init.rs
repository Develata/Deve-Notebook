use std::path::PathBuf;
use deve_core::ledger::Ledger;

pub fn run(ledger_path: &PathBuf, vault_path: &PathBuf, _init_path: PathBuf) -> anyhow::Result<()> {
    println!("Initializing ledger at {:?}...", ledger_path);
    let _ = Ledger::init(ledger_path)?;
    std::fs::create_dir_all(vault_path)?;
    println!("Initialization complete.");
    Ok(())
}
