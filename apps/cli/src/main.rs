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

mod server;
mod commands;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize a new Deve-Note vault
    Init {
        #[arg(short, long, default_value = ".")]
        path: PathBuf,
    },
    /// Scan and index the vault
    Scan,
    /// Watch the vault for changes
    Watch,
    /// Dump ops for a file
    Dump {
        #[arg(short, long)]
        path: String,
    },
    /// Start the backend server
    Serve {
        #[arg(short, long, default_value_t = 3001)]
        port: u16,
    },
    /// Export ledger to JSONL
    Export {
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Verify P2P Sync Logic (Simulation)
    VerifyP2P,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Initialize configuration from Env
    let config = deve_core::config::Config::load();
    
    // Use config values
    let ledger_dir = PathBuf::from(&config.ledger_dir);
    let vault_path = PathBuf::from(&config.vault_path);

    tracing::info!("Starting Deve-Note with profile: {:?}", config.profile);

    match args.command {
        Some(Commands::Init { path }) => commands::init::run(&ledger_dir, &vault_path, path, config.snapshot_depth)?,
        Some(Commands::Scan) => commands::scan::run(&ledger_dir, &vault_path, config.snapshot_depth)?,
        Some(Commands::Watch) => commands::watch::run(&ledger_dir, &vault_path, config.snapshot_depth)?,
        Some(Commands::Dump { path }) => commands::dump::run(&ledger_dir, path, config.snapshot_depth)?,
        Some(Commands::Serve { port }) => commands::serve::run(&ledger_dir, vault_path, port, config.snapshot_depth).await?,
        Some(Commands::Export { output }) => commands::export::run(&ledger_dir, output, config.snapshot_depth)?,
        Some(Commands::VerifyP2P) => commands::verify_p2p::run(config.snapshot_depth)?,
        None => tracing::info!("请提供子命令，使用 --help 查看帮助。"),
    }

    Ok(())
}
