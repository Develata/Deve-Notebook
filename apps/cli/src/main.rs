// apps\cli\src
//! # Deve-Note 命令行应用
//!
//! **架构作用**:
//! 这是 Deve-Note 的命令行入口，提供 Local Hub 和 Backend Server 功能。
//! 遵循 [Deve-Note Plan](../../deve-note%20plan/deve-note%20plan.md) 定义的 Git-Flow P2P 架构。
//!
//! ## 命令说明
//!
//! - `init`: 初始化新的 vault 目录
//! - `scan`: 索引 vault 中的所有 Markdown 文件 (Sync Manager)
//! - `watch`: 监控文件系统变更 (Watcher Service)
//! - `dump`: 调试工具，用于检查 ops 记录
//! - `serve`: 启动 WebSocket 后端服务器 (Backend Architecture)

use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod admin_api;
mod commands;
mod dispatch;
mod dump_support;
mod export_entries;
#[cfg(test)]
mod main_test;
mod server;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// Initialize a new Deve-Note vault
    Init {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
    /// Scan and index the vault
    Scan,
    /// Watch the vault for changes
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
    /// Verify P2P Sync Logic (Simulation)
    VerifyP2P,
    /// Seed a shadow repo with local data
    Seed {
        #[arg(short, long)]
        peer: String,
        #[arg(long)]
        repo: Option<String>,
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
    /// Recover vault files from ledger data
    Recover {
        #[arg(long)]
        repo: Option<String>,
    },
    /// Repair known local corruption from backups and quarantine invalid shadows
    Repair {
        #[arg(long, default_value = "Vault_old/vault")]
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Initialize configuration from Env
    let config = deve_core::config::Config::load_checked()?;
    server::agent_bridge::init_from_config(&config);

    // Use config values
    let ledger_dir = PathBuf::from(&config.ledger_dir);
    let vault_path = PathBuf::from(&config.vault_path);

    tracing::info!("Starting Deve-Note with profile: {:?}", config.profile);

    dispatch::run(args.command, &config, &ledger_dir, &vault_path).await?;
    Ok(())
}
