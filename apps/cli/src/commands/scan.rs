use std::path::PathBuf;
use deve_core::ledger::RepoManager;
use deve_core::vfs::Vfs;

pub fn run(ledger_dir: &PathBuf, vault_path: &PathBuf, snapshot_depth: usize) -> anyhow::Result<()> {
    // Open RepoManager
    let repo = RepoManager::init(ledger_dir, snapshot_depth)?;
    let vfs = Vfs::new(vault_path);
    println!("Scanning vault at {:?}...", vault_path);
    let count = vfs.scan(&repo)?;
    println!("Scanned. Registered {} new documents.", count);
    Ok(())
}
