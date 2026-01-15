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

    // 2. Generate default config.toml
    let config_path = std::path::Path::new("config.toml");
    if !config_path.exists() {
        let default_config = r#"# Deve-Note Configuration

# Application Profile (standard | low-spec)
profile = "standard"

# Path Configuration
# Local ledger storage (contains database and logs)
ledger_dir = "ledger"
# Root directory for your documents
vault_path = "vault"

# P2P Sync Mode (auto | manual)
sync_mode = "auto"

# Performance Tuning
# Number of changes to keep in history
snapshot_depth = 100
# Background compression concurrency
concurrency = 4
"#;
        std::fs::write(config_path, default_config)?;
        println!("Created default 'config.toml'");
    }

    // 3. Generate default .env
    let env_path = std::path::Path::new(".env");
    if !env_path.exists() {
        let default_env = r#"# Deve-Note Environment Overrides
# Uncomment to override config.toml settings

# DEVE_PROFILE=standard
# DEVE_LEDGER_DIR=ledger
# DEVE_VAULT_PATH=vault
# DEVE_SYNC_MODE=auto
"#;
        std::fs::write(env_path, default_env)?;
        println!("Created default '.env'");
    }

    println!("Initialization complete.");
    Ok(())
}
