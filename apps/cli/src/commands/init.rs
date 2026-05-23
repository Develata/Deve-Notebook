// apps\cli\src\commands
//! plan_ref:
//!   - 04_storage#repo-runtime-layout
//!   - 12_commands#cli-commands
//!   - 13_settings#configuration-settings

use deve_core::ledger::RepoManager;
use deve_core::utils::fs::checked_exists;
use std::path::{Path, PathBuf};

/// 初始化命令
///
/// **功能**:
/// 初始化 `ledger`、本地 repo、Projection Locator 和 repo projection workspace 目录结构。
///
/// **参数**:
/// * `ledger_dir`: 账本存储路径
/// * `repo_name`: 本地 repo 名称
/// * `projection_base`: repo projection base；最终 workspace 为 `<projection_base>/<repo_name>`
/// * `path`: 指定的初始化根目录, config.toml 和 .env 将生成在此目录下
/// * `snapshot_depth`: 快照深度配置
pub fn run(
    ledger_dir: &Path,
    repo_name: &str,
    projection_base: &Path,
    path: PathBuf,
    snapshot_depth: usize,
) -> anyhow::Result<()> {
    println!("Initializing ledger at {:?}...", ledger_dir);
    std::fs::create_dir_all(&path)?;
    let repo = RepoManager::init(ledger_dir, snapshot_depth, Some(repo_name), None)?;
    repo.set_projection_base_for_local_repo(repo_name, projection_base)?;
    let workspace_root = repo.local_repo_workspace_root(repo_name)?;
    std::fs::create_dir_all(&workspace_root)?;
    std::fs::create_dir_all(deve_core::utils::notegit::repo_dir(&workspace_root))?;
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&workspace_root)?;
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
# Projection workspaces are configured per repo by:
# deve repo projection set --repo <name-or-uuid> --base <path>

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

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn init_config_template_matches_current_settings_schema() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        run(
            &root.join("ledger"),
            "default",
            &root.join("notes"),
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

    #[test]
    fn init_creates_trinity_workspace_layout() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        run(
            &root.join("ledger"),
            "default",
            &root.join("notes"),
            root.to_path_buf(),
            8,
        )
        .expect("init");

        assert!(root.join("ledger/local").is_dir());
        assert!(root.join("ledger/remotes").is_dir());
        assert!(root.join("ledger/.host/keys").is_dir());
        assert!(root.join("notes/default").is_dir());
        assert!(root.join("notes/default/.notegit").is_dir());
        assert!(root.join("ledger/.host/projection-locators.toml").is_file());
        let gitignore = std::fs::read_to_string(root.join("notes/default/.gitignore"))
            .expect("repo-local gitignore");
        assert!(gitignore.lines().any(|line| line.trim() == ".notegit/"));
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
            "default",
            &root.join("notes"),
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
