use std::path::PathBuf;
use deve_core::ledger::Ledger;
use deve_core::vfs::Vfs;

pub fn run(ledger_path: &PathBuf, vault_path: &PathBuf, snapshot_depth: usize) -> anyhow::Result<()> {
    // Open Ledger
    let ledger = Ledger::init(ledger_path, snapshot_depth)?;
    let vfs = Vfs::new(vault_path);
    println!("Scanning vault at {:?}...", vault_path);
    let count = vfs.scan(&ledger)?;
    println!("Scanned. Registered {} new documents.", count);
    Ok(())
}
