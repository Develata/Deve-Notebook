use crate::Commands;
use crate::commands;
use std::path::{Path, PathBuf};

pub async fn run(
    command: Option<Commands>,
    config: &deve_core::config::Config,
    ledger_dir: &PathBuf,
    vault_path: &Path,
) -> anyhow::Result<()> {
    match command {
        Some(Commands::Init { path }) => {
            commands::init::run(ledger_dir, vault_path, path, config.snapshot_depth)?
        }
        Some(Commands::Scan) => commands::scan::run(ledger_dir, vault_path, config.snapshot_depth)?,
        Some(Commands::Watch) => {
            commands::watch::run(ledger_dir, vault_path, config.snapshot_depth)?
        }
        Some(Commands::Dump { path, repo }) => {
            commands::dump::run(ledger_dir, path, repo, config.snapshot_depth)?
        }
        Some(Commands::Serve { port, dev }) => {
            commands::serve::run(
                ledger_dir,
                vault_path.to_path_buf(),
                port,
                config.snapshot_depth,
                dev,
            )
            .await?
        }
        Some(Commands::Export { output, repo }) => {
            commands::export::run(ledger_dir, output, repo, config.snapshot_depth)?
        }
        Some(Commands::VerifyP2P) => commands::verify_p2p::run(config.snapshot_depth)?,
        Some(Commands::Seed { peer, repo }) => {
            commands::seed::run(ledger_dir, peer, repo, config.snapshot_depth)?
        }
        Some(Commands::NodeCheck { repair, repo }) => {
            commands::node_check::run(ledger_dir, config.snapshot_depth, repair, repo)?
        }
        Some(Commands::Repair {
            backup,
            repo,
            paths,
            rebuild_projection,
        }) => commands::repair::run(
            ledger_dir,
            vault_path,
            &backup,
            config.snapshot_depth,
            repo.as_deref(),
            &paths,
            rebuild_projection,
        )?,
        None => tracing::info!("请提供子命令，使用 --help 查看帮助。"),
    }
    Ok(())
}
