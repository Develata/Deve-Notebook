use std::path::PathBuf;
use deve_core::ledger::RepoManager;
use deve_core::vfs::Vfs;

/// 扫描命令
///
/// **功能**:
/// 遍历 `vault` 目录，将所有 Markdown 文件注册到 `ledger` 中。
/// 使用 `Vfs` 模块执行扫描操作。
pub fn run(ledger_dir: &PathBuf, vault_path: &PathBuf, snapshot_depth: usize) -> anyhow::Result<()> {
    // 初始化 RepoManager
    let repo = RepoManager::init(ledger_dir, snapshot_depth, None)?;
    let vfs = Vfs::new(vault_path);
    println!("Scanning vault at {:?}...", vault_path);
    let count = vfs.scan(&repo)?;
    println!("Scanned. Registered {} new documents.", count);
    Ok(())
}
