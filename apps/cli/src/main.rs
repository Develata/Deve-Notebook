// apps\cli\src
//! # Deve-Note 命令行应用
//! plan_ref:
//!   - 14_commands#cli-commands
//!
//! **架构作用**:
//! 这是 Deve-Note 的命令行入口，提供 Local Hub 和 Backend Server 功能。
//! 遵循 [Deve-Note Plan](../../deve-note%20plan/deve-note%20plan.md) 定义的 Git-Flow P2P 架构。
//!
//! ## 命令说明
//!
//! - `init`: 初始化 ledger 与 repo Projection Locator
//! - `scan`: 索引 repo projection workspace 中的 Markdown 文件 (Sync Manager)
//! - `watch`: 监控 repo projection workspace 变更 (Watcher Service)
//! - `dump`: 调试工具，用于检查 ops 记录
//! - `serve`: 启动 WebSocket 后端服务器 (Backend Architecture)
//! - `graph`: 输出当前 repo 的只读 GraphProjection JSON
//! - `git status`: 检查 Git ecosystem mirror bridge 状态
//! - `git mirror`: 手动执行 queued Git mirror commits
//! - `git export`: 将 queued Deve commits 导出到 Git mirror
//! - `git import`: 规划或显式 apply 外部 Git/worktree changes
//! - `git push`: 将 Git mirror 发布到远端
//! - `backup bind/inspect/list/verify/run/restore/unbind`: 规划 WebDAV/S3 backup binding，或只读检查 locator、provider adapter plan、branch manifest discovery、remote layout 与 restore candidate admission

use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod admin_api;
mod commands;
mod dispatch;
mod dump_support;
mod export_entries;
mod graph_projection;
#[cfg(test)]
mod main_test;
mod repo_init;
mod server;

pub(crate) use commands::backup::BackupAction;
pub(crate) use commands::sc::ScAction;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialize a ledger repo and Projection Locator
    Init {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        repo: String,
        #[arg(long = "projection-base")]
        projection_base: PathBuf,
        #[arg(long = "repo-id")]
        repo_id: Option<uuid::Uuid>,
        #[arg(long = "repo-url")]
        repo_url: Option<String>,
    },
    /// Scan repo projection workspaces
    Scan,
    /// Watch repo projection workspaces for changes
    Watch {
        #[arg(long)]
        dry_run: bool,
    },
    /// Dump ops for a file
    Dump {
        #[arg(short, long)]
        path: String,
        #[arg(long)]
        repo: Option<String>,
    },
    /// Start the backend server
    Serve {
        #[arg(short, long, default_value_t = 3001)]
        port: u16,
        #[arg(long)]
        dev: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, hide = true)]
        native_loopback: bool,
    },
    /// Export ledger to JSONL or Markdown
    Export {
        #[arg(short, long, visible_alias = "out")]
        output: Option<String>,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        doc: Option<String>,
        #[arg(long, default_value = "json")]
        format: String,
        #[arg(long)]
        allow_degraded_projection: bool,
    },
    /// Print repo-scoped read-only graph projection JSON
    Graph {
        #[arg(long)]
        repo: Option<String>,
        #[arg(short, long, visible_alias = "out")]
        output: Option<String>,
        #[arg(long)]
        pretty: bool,
        #[arg(long)]
        allow_degraded_projection: bool,
    },
    /// Verify P2P Sync Logic (Simulation or live shadow check)
    VerifyP2P {
        #[arg(long = "live-ledger-dir")]
        live_ledger_dir: Option<PathBuf>,
        #[arg(long = "repo-id")]
        repo_id: Option<uuid::Uuid>,
        #[arg(long = "peer-id")]
        peer_id: Option<String>,
        #[arg(long = "doc-id")]
        doc_id: Option<uuid::Uuid>,
        #[arg(long = "contains")]
        contains: Option<String>,
        #[arg(long = "local-must-not-contain")]
        local_must_not_contain: Option<String>,
    },
    /// Seed a shadow repo with local data
    Seed {
        #[arg(short, long)]
        peer: String,
        #[arg(long)]
        repo: Option<String>,
    },
    /// Seed a deterministic merge conflict fixture for browser smoke tests
    #[command(hide = true)]
    SeedMergeConflictFixture {
        #[arg(long, default_value = "peer-a")]
        peer: String,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long, default_value = "notes/conflict.md")]
        path: String,
        #[arg(long, default_value = "base")]
        base: String,
        #[arg(long, default_value = "local")]
        local: String,
        #[arg(long, default_value = "remote")]
        remote: String,
    },
    /// Check node consistency
    NodeCheck {
        #[arg(long)]
        repair: bool,
        #[arg(long, conflicts_with = "repair")]
        projection: bool,
        #[arg(long)]
        repo: Option<String>,
    },
    /// Recover projection workspace files from ledger data
    Recover {
        #[arg(long)]
        repo: Option<String>,
    },
    /// Print source-control staged and unstaged counts
    ScStatus {
        #[arg(long)]
        repo: Option<String>,
    },
    /// Inspect or mutate Deve Source Control state
    Sc {
        #[command(subcommand)]
        action: ScAction,
    },
    /// Inspect Git ecosystem mirror bridge state
    Git {
        #[command(subcommand)]
        action: GitAction,
    },
    /// Inspect backup locator state without network access
    Backup {
        #[command(subcommand)]
        action: BackupAction,
    },
    /// Inspect or update repo Projection Locators
    Repo {
        #[command(subcommand)]
        action: RepoAction,
    },
    /// Repair known local corruption from backups and quarantine invalid shadows
    Repair {
        /// Run repair readiness checks without executing repair steps
        #[arg(long)]
        check: bool,
        /// Repo-scoped Markdown backup root used by --path restore
        #[arg(long, default_value = "backups/projection-workspace")]
        backup: PathBuf,
        #[arg(long)]
        repo: Option<String>,
        #[arg(long = "path")]
        paths: Vec<String>,
        #[arg(long)]
        rebuild_projection: bool,
    },
    /// Print or update config.toml
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ConfigAction {
    /// Print effective runtime configuration as TOML
    Print,
    /// Set a whitelisted key in config.toml
    Set { key: String, value: String },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RepoAction {
    /// Inspect or update repo Projection Locator state
    Projection {
        #[command(subcommand)]
        action: RepoProjectionAction,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum RepoProjectionAction {
    /// Set the projection base for a local repo
    Set {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        base: PathBuf,
    },
    /// List repo Projection Locators
    List,
    /// Check the resolved projection workspace root for a local repo
    Check {
        #[arg(long)]
        repo: String,
    },
    /// List unexplained projection/workspace drift
    Drift {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum GitAction {
    /// Print read-only Git mirror bridge status
    Status {
        #[arg(long)]
        repo: Option<String>,
    },
    /// Execute queued Git mirror commit records
    #[command(visible_alias = "flush")]
    Mirror {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        retry_out_of_sync: bool,
    },
    /// Export queued Deve commits into the Git mirror
    Export {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        retry_out_of_sync: bool,
    },
    /// Plan or explicitly apply external Git/worktree changes into pending/import
    Import {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        apply: bool,
    },
    /// Push the exported Git mirror to a remote
    Push {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        remote: Option<String>,
        #[arg(long)]
        branch: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt::init();

    if run_pre_config_command(&args.command)? {
        return Ok(());
    }

    // Initialize configuration from Env
    let config = deve_core::config::Config::load_checked()?;
    server::ai_chat::init_from_config(&config);
    server::agent_bridge::init_from_config(&config);

    // Use config values
    let ledger_dir = PathBuf::from(&config.ledger_dir);

    tracing::info!("Starting Deve-Note with profile: {:?}", config.profile);

    dispatch::run(args.command, &config, &ledger_dir).await?;
    Ok(())
}

fn run_pre_config_command(command: &Option<Commands>) -> anyhow::Result<bool> {
    if let Some(Commands::Init {
        path,
        repo,
        projection_base,
        repo_id,
        repo_url,
    }) = command
    {
        commands::init::run(
            &path.join("ledger"),
            repo,
            projection_base,
            path.to_path_buf(),
            100,
            *repo_id,
            repo_url.clone(),
        )?;
        return Ok(true);
    }
    if let Some(Commands::Config {
        action: ConfigAction::Set { key, value },
    }) = command
    {
        commands::config::set(key, value)?;
        return Ok(true);
    }
    Ok(false)
}
