//! plan_ref:
//!   - 12_commands#cli-commands

use crate::commands;
use crate::{Commands, ConfigAction, GitAction};
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
            allow_degraded_projection,
        }) => commands::export::run(
            ledger_dir,
            output,
            repo,
            doc,
            config.snapshot_depth,
            &format,
            allow_degraded_projection,
        )?,
        Some(Commands::Graph {
            repo,
            output,
            pretty,
            allow_degraded_projection,
        }) => commands::graph::run(
            ledger_dir,
            repo.as_deref(),
            output,
            pretty,
            allow_degraded_projection,
            config.snapshot_depth,
        )?,
        Some(Commands::Recover { repo }) => {
            commands::recover::run(ledger_dir, vault_path, repo, config.snapshot_depth)?
        }
        Some(Commands::ScStatus { repo }) => {
            commands::sc_status::run(ledger_dir, repo.as_deref(), config.snapshot_depth)?
        }
        Some(Commands::Git { action }) => match action {
            GitAction::Status { repo } => commands::git::status(
                ledger_dir,
                vault_path,
                repo.as_deref(),
                config.snapshot_depth,
            )?,
            GitAction::Mirror {
                repo,
                retry_out_of_sync,
            } => commands::git::mirror(
                ledger_dir,
                vault_path,
                repo.as_deref(),
                retry_out_of_sync,
                config.snapshot_depth,
            )?,
            GitAction::Export {
                repo,
                retry_out_of_sync,
            } => commands::git::export(
                ledger_dir,
                vault_path,
                repo.as_deref(),
                retry_out_of_sync,
                config.snapshot_depth,
            )?,
            GitAction::Import { repo } => commands::git::import(
                ledger_dir,
                vault_path,
                repo.as_deref(),
                config.snapshot_depth,
            )?,
        },
        Some(Commands::VerifyP2P) => commands::verify_p2p::run(config.snapshot_depth)?,
        Some(Commands::Seed { peer, repo }) => {
            commands::seed::run(ledger_dir, peer, repo, config.snapshot_depth)?
        }
        Some(Commands::NodeCheck {
            repair,
            projection,
            repo,
        }) => commands::node_check::run(
            ledger_dir,
            vault_path,
            config.snapshot_depth,
            repair,
            projection,
            repo,
        )?,
        Some(Commands::Repair {
            check,
            backup,
            repo,
            paths,
            rebuild_projection,
        }) => commands::repair::run(
            ledger_dir,
            vault_path,
            config.snapshot_depth,
            commands::repair::RepairOptions {
                backup_root: &backup,
                target_repo: repo.as_deref(),
                paths: &paths,
                rebuild_projection,
                check,
            },
        )?,
        Some(Commands::Config { action }) => match action {
            ConfigAction::Print => commands::config::print(config)?,
            ConfigAction::Set { key, value } => commands::config::set(&key, &value)?,
        },
        None => tracing::info!("请提供子命令，使用 --help 查看帮助。"),
    }
    Ok(())
}
