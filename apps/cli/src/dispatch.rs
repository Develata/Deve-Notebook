use crate::commands;
use crate::{Commands, ConfigAction};
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
        Some(Commands::Watch { dry_run }) => {
            commands::watch::run(ledger_dir, vault_path, config.snapshot_depth, dry_run)?
        }
        Some(Commands::Dump { path, repo }) => {
            commands::dump::run(ledger_dir, path, repo, config.snapshot_depth)?
        }
        Some(Commands::Serve { port, dev, dry_run }) => {
            commands::serve::run(
                ledger_dir,
                vault_path.to_path_buf(),
                commands::serve::ServeOptions {
                    port,
                    snapshot_depth: config.snapshot_depth,
                    dev,
                    dry_run,
                    profile: config.profile,
                    sync_mode: config.sync_mode,
                },
            )
            .await?
        }
        Some(Commands::Export {
            output,
            repo,
            doc,
            format,
        }) => commands::export::run(
            ledger_dir,
            output,
            repo,
            doc,
            config.snapshot_depth,
            &format,
        )?,
        Some(Commands::Recover { repo }) => {
            commands::recover::run(ledger_dir, vault_path, repo, config.snapshot_depth)?
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
        Some(Commands::Config { action }) => match action {
            ConfigAction::Print => commands::config::print(config)?,
            ConfigAction::Set { key, value } => commands::config::set(&key, &value)?,
        },
        None => tracing::info!("请提供子命令，使用 --help 查看帮助。"),
    }
    Ok(())
}
