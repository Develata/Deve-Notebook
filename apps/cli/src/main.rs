//! # Deve-Note 命令行应用
//!
//! 这是 Deve-Note 的命令行界面，提供开发工具和后端服务器。
//!
//! ## 命令说明
//!
//! - `init`: 初始化新的 vault 目录
//! - `scan`: 索引 vault 中的所有 Markdown 文件
//! - `watch`: 监控文件系统变更
//! - `dump`: 调试工具，用于检查文档操作记录
//! - `serve`: 启动 WebSocket 后端服务器

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Default paths
    let ledger_path = PathBuf::from("deve.db");
    let vault_path = PathBuf::from("vault");

    match args.command {
        Some(Commands::Init { path }) => commands::init::run(&ledger_path, &vault_path, path)?,
        Some(Commands::Scan) => commands::scan::run(&ledger_path, &vault_path)?,
        Some(Commands::Watch) => commands::watch::run(&ledger_path, &vault_path)?,
        Some(Commands::Dump { path }) => commands::dump::run(&ledger_path, path)?,
        Some(Commands::Serve { port }) => commands::serve::run(&ledger_path, vault_path, port).await?,
        None => println!("Please provide a subcommand. Try --help."),
    }

    Ok(())
}
