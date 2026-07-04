//! plan_ref:
//!   - 14_commands#cli-commands

use crate::commands;
use crate::{Commands, ConfigAction, NgitAction, RepoAction, RepoProjectionAction, ScAction};
use std::path::PathBuf;

mod backup;

pub async fn run(
    command: Option<Commands>,
    config: &deve_core::config::Config,
    ledger_dir: &PathBuf,
) -> anyhow::Result<()> {
    match command {
        Some(Commands::Init {
            path,
            repo,
            projection_base,
            repo_id,
            repo_url,
        }) => commands::init::run(
            ledger_dir,
            &repo,
            &projection_base,
            path,
            config.snapshot_depth,
            repo_id,
            repo_url,
        )?,
        Some(Commands::Scan) => commands::scan::run(ledger_dir, config.snapshot_depth)?,
        Some(Commands::Watch { dry_run }) => {
            commands::watch::run(ledger_dir, config.snapshot_depth, dry_run)?
        }
        Some(Commands::Dump { path, repo }) => {
            commands::dump::run(ledger_dir, path, repo, config.snapshot_depth)?
        }
        Some(Commands::Serve {
            port,
            dev,
            dry_run,
            native_loopback,
        }) => {
            commands::serve::run(
                ledger_dir,
                commands::serve::ServeOptions {
                    port,
                    snapshot_depth: config.snapshot_depth,
                    dev,
                    dry_run,
                    profile: config.profile,
                    sync_mode: config.sync_mode,
                    p2p: config.p2p.clone(),
                    native_loopback,
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
            commands::recover::run(ledger_dir, repo, config.snapshot_depth)?
        }
        Some(Commands::ScStatus { repo }) => {
            commands::sc_status::run(ledger_dir, repo.as_deref(), config.snapshot_depth)?
        }
        Some(Commands::Sc { action }) => match action {
            ScAction::Status { repo } => {
                commands::sc::status(ledger_dir, repo.as_deref(), config.snapshot_depth)?
            }
            ScAction::Stage { repo, all } => {
                commands::sc::stage(ledger_dir, repo.as_deref(), all, config.snapshot_depth)?
            }
            ScAction::Commit { repo, message } => {
                commands::sc::commit(ledger_dir, repo.as_deref(), &message, config.snapshot_depth)?
            }
        },
        Some(Commands::Ngit { action }) => match action {
            NgitAction::Status { repo } => {
                commands::git::status(ledger_dir, repo.as_deref(), config.snapshot_depth)?
            }
            NgitAction::Mirror {
                repo,
                retry_out_of_sync,
            } => commands::git::mirror(
                ledger_dir,
                repo.as_deref(),
                retry_out_of_sync,
                config.snapshot_depth,
            )?,
            NgitAction::Export {
                repo,
                retry_out_of_sync,
            } => commands::git::export(
                ledger_dir,
                repo.as_deref(),
                retry_out_of_sync,
                config.snapshot_depth,
            )?,
            NgitAction::Import { repo, apply } => {
                commands::git::import(ledger_dir, repo.as_deref(), apply, config.snapshot_depth)?
            }
            NgitAction::Push {
                repo,
                remote,
                branch,
            } => commands::git::push(
                ledger_dir,
                repo.as_deref(),
                remote.as_deref(),
                branch.as_deref(),
                config.snapshot_depth,
            )?,
        },
        Some(Commands::ProjectionRemote { action }) => {
            commands::projection_remote::run(ledger_dir, action, config.snapshot_depth)?
        }
        Some(Commands::Backup { action }) => backup::run(action)?,
        Some(Commands::VerifyP2P {
            live_ledger_dir,
            repo_id,
            peer_id,
            doc_id,
            contains,
            local_must_not_contain,
        }) => match live_ledger_dir {
            Some(ledger_dir) => commands::verify_p2p::run_live_shadow_check(
                commands::verify_p2p::LiveShadowCheckOptions {
                    ledger_dir,
                    repo_id,
                    peer_id,
                    doc_id,
                    contains,
                    local_must_not_contain,
                },
                config.snapshot_depth,
            )?,
            None => commands::verify_p2p::run(config.snapshot_depth)?,
        },
        Some(Commands::Seed { peer, repo }) => {
            commands::seed::run(ledger_dir, peer, repo, config.snapshot_depth)?
        }
        Some(Commands::SeedMergeConflictFixture {
            peer,
            repo,
            path,
            base,
            local,
            remote,
        }) => commands::merge_conflict_fixture::run(
            ledger_dir,
            config.snapshot_depth,
            commands::merge_conflict_fixture::MergeConflictFixtureOptions {
                peer,
                repo,
                path,
                base,
                local,
                remote,
            },
        )?,
        Some(Commands::NodeCheck {
            repair,
            projection,
            repo,
        }) => {
            commands::node_check::run(ledger_dir, config.snapshot_depth, repair, projection, repo)?
        }
        Some(Commands::Repair {
            check,
            backup,
            repo,
            paths,
            rebuild_projection,
        }) => commands::repair::run(
            ledger_dir,
            config.snapshot_depth,
            commands::repair::RepairOptions {
                backup_root: &backup,
                target_repo: repo.as_deref(),
                paths: &paths,
                rebuild_projection,
                check,
            },
        )?,
        Some(Commands::Repo { action }) => match action {
            RepoAction::Projection { action } => match action {
                RepoProjectionAction::Set { repo, base } => {
                    commands::repo_projection::set(ledger_dir, &repo, &base, config.snapshot_depth)?
                }
                RepoProjectionAction::List => {
                    commands::repo_projection::list(ledger_dir, config.snapshot_depth)?
                }
                RepoProjectionAction::Check { repo } => {
                    commands::repo_projection::check(ledger_dir, &repo, config.snapshot_depth)?
                }
                RepoProjectionAction::Drift { repo, root } => commands::repo_projection::drift(
                    ledger_dir,
                    &repo,
                    root.as_deref(),
                    config.snapshot_depth,
                )?,
            },
        },
        Some(Commands::Config { action }) => match action {
            ConfigAction::Print => commands::config::print(config)?,
            ConfigAction::Set { key, value } => commands::config::set(&key, &value)?,
        },
        None => tracing::info!("请提供子命令，使用 --help 查看帮助。"),
    }
    Ok(())
}
