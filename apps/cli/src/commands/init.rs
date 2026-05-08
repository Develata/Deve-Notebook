// apps\cli\src\commands
//! plan_ref:
//!   - 04_storage#repo-runtime-layout
//!   - 12_commands#cli-commands
//!   - 13_settings#configuration-settings

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
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&vault_path.join("default"))?;
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

# Merge Strategy (manual | auto)
merge_strategy = "manual"

# Performance Tuning
# Number of changes to keep in history
snapshot_depth = 100
# Memory cache limit in MB
mem_cache_mb = 128
# Background compression concurrency
concurrency = 4

[ui]
locale = "auto"
theme = "auto"
sidebar_visible = true
statusbar_visible = true
outline_visible = true
outline_width = 260
sidebar_width = 250
right_panel_width = 350
outer_gutter = 16
recent_commands_count = 3
recent_docs_count = 10

[ai]
mode = "native"
native_enabled = true

[ai.agent_bridge]
enabled = false
trusted = false
timeout_ms = 30000
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
# AI_API_KEY=
# AI_BASE_URL=https://api.openai.com/v1
# AI_MODEL=gpt-4o-mini
# AGENT_CLI_PATH=
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

    #[test]
    fn init_config_template_matches_current_settings_schema() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        run(
            &root.join("ledger"),
            &root.join("vault"),
            root.to_path_buf(),
            8,
        )
        .expect("init");

        let config = std::fs::read_to_string(root.join("config.toml")).expect("config");
        assert!(config.contains("merge_strategy = \"manual\""));
        assert!(config.contains("mem_cache_mb = 128"));
        assert!(config.contains("[ui]"));
        assert!(config.contains("recent_docs_count = 10"));
        assert!(config.contains("[ai]"));
        assert!(config.contains("mode = \"native\""));
        assert!(config.contains("[ai.agent_bridge]"));
        assert!(config.contains("timeout_ms = 30000"));
    }

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
