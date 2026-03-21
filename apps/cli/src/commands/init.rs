// apps\cli\src\commands
use deve_core::ledger::RepoManager;
use std::path::{Path, PathBuf};

/// 初始化命令
///
/// **功能**:
/// 初始化 `ledger` 和 `vault` 目录结构。
///
/// **参数**:
/// * `ledger_dir`: 账本存储路径
/// * `vault_path`: 文档库路径
/// * `path`: 指定的初始化根目录, config.toml 和 .env 将生成在此目录下
/// * `snapshot_depth`: 快照深度配置
pub fn run(
    ledger_dir: &Path,
    vault_path: &Path,
    path: PathBuf,
    snapshot_depth: usize,
) -> anyhow::Result<()> {
    println!("Initializing ledger at {:?}...", ledger_dir);
    // 1. 初始化 RepoManager (创建目录结构)
    let _ = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    std::fs::create_dir_all(vault_path.join("default"))?;
    std::fs::create_dir_all(deve_core::utils::notegit::repo_dir(
        &vault_path.join("default"),
    ))?;
    std::fs::create_dir_all(deve_core::utils::notegit::host_keys_dir(ledger_dir))?;

    // 2. Generate default config.toml
    let config_path = path.join("config.toml");
    if !checked_exists(&config_path, "config.toml target")? {
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
    let env_path = path.join(".env");
    if !checked_exists(&env_path, ".env target")? {
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

fn checked_exists(path: &Path, context: &str) -> anyhow::Result<bool> {
    path.try_exists()
        .map_err(|err| anyhow::anyhow!("Failed to stat {} {:?}: {}", context, path, err))
}

#[cfg(test)]
mod tests {
    use super::run;

    #[cfg(unix)]
    #[test]
    fn init_fails_closed_on_unreadable_config_target() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("workspace");
        std::fs::create_dir_all(&root).expect("root");
        let bad_parent = root.join("config-base");
        std::fs::write(&bad_parent, "not-a-dir").expect("bad parent");

        let err = run(
            &root.join("ledger"),
            &root.join("vault"),
            bad_parent.clone(),
            8,
        )
        .expect_err("unreadable config target must fail closed");

        assert!(
            err.to_string()
                .contains("Failed to stat config.toml target")
                || err.to_string().contains("Not a directory")
        );
    }
}
