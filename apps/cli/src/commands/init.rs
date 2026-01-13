use std::path::PathBuf;
use deve_core::ledger::RepoManager;

/// 初始化命令
///
/// **功能**:
/// 初始化 `ledger` 和 `vault` 目录结构。
///
/// **参数**:
/// * `ledger_dir`: 账本存储路径
/// * `vault_path`: 文档库路径
/// * `path`: 指定的初始化路径 (可能是 ledger 或 vault，取决于上下文，此处似乎未使用)
/// * `snapshot_depth`: 快照深度配置
pub fn run(ledger_dir: &PathBuf, vault_path: &PathBuf, path: PathBuf, snapshot_depth: usize) -> anyhow::Result<()> {
    println!("Initializing ledger at {:?}...", ledger_dir);
    // 1. 初始化 RepoManager (创建目录结构)
    let _ = RepoManager::init(ledger_dir, snapshot_depth)?;
    std::fs::create_dir_all(vault_path)?;
    println!("Initialization complete.");
    Ok(())
}
