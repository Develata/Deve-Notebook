// apps\cli\src\commands
//! plan_ref:
//!   - 03_storage/index#repo-runtime-layout
//!   - 14_commands#cli-commands
//!   - 15_settings#configuration-settings

use deve_core::ledger::RepoManager;
use deve_core::ledger::init::RepoInitOptions;
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
/// * `projection_base`: repo projection base；最终 workspace 为 `<projection_base>/<safe_repo_name>--<repo_id>`
/// * `path`: 指定的初始化根目录, config.toml 和 .env 将生成在此目录下
/// * `snapshot_depth`: 快照深度配置
/// * `repo_id`: 可选显式 RepoId；只允许用于新 repo 或匹配既有 metadata 的 repo
/// * `repo_url`: 可选 repo URL；显式提供时必须匹配既有 metadata
pub fn run(
    ledger_dir: &Path,
    repo_name: &str,
    projection_base: &Path,
    path: PathBuf,
    snapshot_depth: usize,
    repo_id: Option<uuid::Uuid>,
    repo_url: Option<String>,
) -> anyhow::Result<()> {
    println!("Initializing ledger at {:?}...", ledger_dir);
    std::fs::create_dir_all(&path)?;
    let repo = RepoManager::init_with_options(
        ledger_dir,
        snapshot_depth,
        Some(repo_name),
        RepoInitOptions { repo_id, repo_url },
    )?;
    repo.set_projection_base_for_local_repo(repo_name, projection_base)?;
    let workspace_root = repo.local_repo_workspace_root(repo_name)?;
    std::fs::create_dir_all(&workspace_root)?;
    repo.ensure_local_repo_workspace_identity(repo_name)?;
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

[source_control]
# Git bridge mode (mirror | off). Deve ledger/.notegit remains the authority.
git_bridge = "mirror"

# Performance Tuning
# Number of changes to keep in history
snapshot_depth = 100
# Memory cache limit in MB
mem_cache_mb = 128
# Background compression concurrency
concurrency = 4

[p2p]
enabled = false
inbound_token_env = "DEVE_P2P_INBOUND_TOKEN"
connect_interval_ms = 5000
# [[p2p.peers]]
# label = "peer-b"
# peer_id = "peer-b"
# repo_id = "11111111-1111-1111-1111-111111111111"
# ws_url = "ws://127.0.0.1:3102/ws"
# auth_token_env = "DEVE_P2P_PEER_B_TOKEN"
# enabled = true

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
    use deve_core::ledger::RepoManager;

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
            None,
            None,
        )
        .expect("init");

        let config = std::fs::read_to_string(root.join("config.toml")).expect("config");
        assert!(config.contains("merge_strategy = \"manual\""));
        assert!(config.contains("[source_control]"));
        assert!(config.contains("git_bridge = \"mirror\""));
        assert!(config.contains("[p2p]"));
        assert!(config.contains("enabled = false"));
        assert!(config.contains("inbound_token_env = \"DEVE_P2P_INBOUND_TOKEN\""));
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
            None,
            None,
        )
        .expect("init");

        assert!(root.join("ledger/local").is_dir());
        assert!(root.join("ledger/remotes").is_dir());
        assert!(root.join("ledger/.host/keys").is_dir());
        let mut workspaces = std::fs::read_dir(root.join("notes"))
            .expect("notes dir")
            .map(|entry| entry.expect("workspace entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("default--"))
            })
            .collect::<Vec<_>>();
        workspaces.sort();
        assert_eq!(workspaces.len(), 1);
        let workspace = &workspaces[0];
        assert!(workspace.is_dir());
        assert!(workspace.join(".notegit").is_dir());
        assert!(workspace.join(".notegit/identity.toml").is_file());
        assert!(root.join("ledger/.host/projection-locators.toml").is_file());
        let gitignore =
            std::fs::read_to_string(workspace.join(".gitignore")).expect("repo-local gitignore");
        assert!(gitignore.lines().any(|line| line.trim() == ".notegit/"));
    }

    #[test]
    fn init_accepts_explicit_repo_id_for_new_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let repo_id = uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid");

        run(
            &root.join("ledger"),
            "default",
            &root.join("notes"),
            root.to_path_buf(),
            8,
            Some(repo_id),
            Some("urn:test:default".to_string()),
        )
        .expect("init");

        let repo = RepoManager::init(
            &root.join("ledger"),
            8,
            Some("default"),
            Some("urn:test:default"),
        )
        .expect("reopen");
        assert_eq!(
            repo.get_repo_info().expect("repo info").expect("repo").uuid,
            repo_id
        );
    }

    #[test]
    fn init_repo_id_mismatch_fails_closed_for_existing_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        run(
            &root.join("ledger"),
            "default",
            &root.join("notes"),
            root.to_path_buf(),
            8,
            Some(uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").expect("uuid")),
            Some("urn:test:default".to_string()),
        )
        .expect("init");

        let err = run(
            &root.join("ledger"),
            "default",
            &root.join("notes"),
            root.to_path_buf(),
            8,
            Some(uuid::Uuid::parse_str("22222222-2222-2222-2222-222222222222").expect("uuid")),
            Some("urn:test:default".to_string()),
        )
        .expect_err("repo id mismatch must fail closed");

        assert!(
            err.to_string()
                .contains("explicit repo-id init fails closed")
        );
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
            None,
            None,
        )
        .expect_err("unreadable config target must fail closed");

        assert!(
            err.to_string()
                .contains("Failed to stat config.toml target")
                || err.to_string().contains("Not a directory")
                || err.to_string().contains("File exists")
        );
    }
}
