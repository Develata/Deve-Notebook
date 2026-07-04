//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#cli-commands
//!
//! CLI shell for remote Markdown projection transport intents.

mod webdav;

use crate::commands::repo_arg::resolve_local_repo_args;
use crate::commands::source_control_workspace_gate::ensure_local_repo_workspace_identity_for_write;
use anyhow::{Result, bail};
use clap::Subcommand;
use deve_core::ledger::RepoManager;
use deve_core::remote_projection::{
    RemoteProjectionDirection, RemoteProjectionPlanInput, RemoteProjectionProvider,
    plan_remote_projection_transport,
};
use std::path::Path;

#[derive(Subcommand, Debug)]
pub(crate) enum ProjectionRemoteAction {
    /// Push or pull through a WebDAV projection transport
    Webdav {
        #[command(subcommand)]
        action: ProjectionRemoteDirectionAction,
    },
    /// Push or pull through an S3 projection transport
    S3 {
        #[command(subcommand)]
        action: ProjectionRemoteDirectionAction,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ProjectionRemoteDirectionAction {
    /// Upload the Markdown projection folder
    Push {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        locator: String,
    },
    /// Download and overwrite the Markdown projection folder
    Pull {
        #[arg(long)]
        repo: Option<String>,
        #[arg(long)]
        locator: String,
    },
}

pub fn run(ledger_dir: &Path, action: ProjectionRemoteAction, snapshot_depth: usize) -> Result<()> {
    let mut webdav_provider = webdav::WebDavProjectionProvider::new()?;
    run_with_provider(ledger_dir, action, snapshot_depth, &mut webdav_provider)
}

fn run_with_provider(
    ledger_dir: &Path,
    action: ProjectionRemoteAction,
    snapshot_depth: usize,
    webdav_provider: &mut dyn webdav::WebDavProjectionPushAdapter,
) -> Result<()> {
    let request = request_from_action(action);
    let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
        provider: request.provider,
        direction: request.direction,
        locator: request.locator.clone(),
    })?;
    let provider_direction_wired = provider_direction_wired_for(&request);

    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_names = resolve_local_repo_args(&repo, request.repo.as_deref())?;
    if provider_direction_wired && request.repo.is_none() && repo_names.len() != 1 {
        bail!(
            "remote projection provider I/O requires an explicit --repo when multiple local repos are present"
        );
    }
    for repo_name in repo_names {
        ensure_local_repo_workspace_identity_for_write(
            &repo,
            &repo_name,
            "remote projection transport",
        )?;
        let workspace = repo.local_repo_workspace_root(&repo_name)?;
        if provider_direction_wired {
            let files = webdav::collect_markdown_projection_files(&workspace)?;
            println!(
                "projection_remote[{repo_name}]: provider={} direction={} scope={} workspace={} writes_ledger={} external_changes_confirmation_required={} provider_direction_wired=true provider_io_ready=false planned_files={}",
                plan.provider.as_str(),
                plan.direction.as_str(),
                plan.projection_scope,
                workspace.display(),
                plan.writes_ledger,
                plan.external_changes_confirmation_required,
                files.len(),
            );
            let outcome =
                webdav_provider.push_projection_files(request.provider, &plan.locator, &files)?;
            println!(
                "projection_remote[{repo_name}]: provider={} direction={} scope={} workspace={} writes_ledger={} external_changes_confirmation_required={} provider_io_ready=true uploaded_files={} writes_source_control_staging={} writes_commit_anchor={} writes_git_main_mirror={} provider_metadata_diagnostic_only={}",
                plan.provider.as_str(),
                plan.direction.as_str(),
                plan.projection_scope,
                workspace.display(),
                plan.writes_ledger,
                plan.external_changes_confirmation_required,
                outcome.uploaded_files,
                outcome.effects.writes_source_control_staging,
                outcome.effects.writes_commit_anchor,
                outcome.effects.writes_git_main_mirror,
                outcome.provider_metadata_is_diagnostic_only,
            );
        } else {
            println!(
                "projection_remote[{repo_name}]: provider={} direction={} scope={} workspace={} writes_ledger={} external_changes_confirmation_required={} provider_io_ready=false",
                plan.provider.as_str(),
                plan.direction.as_str(),
                plan.projection_scope,
                workspace.display(),
                plan.writes_ledger,
                plan.external_changes_confirmation_required,
            );
        }
    }
    if provider_direction_wired {
        return Ok(());
    }
    bail!(
        "remote projection provider I/O is not wired yet (provider_io_ready=false); no projection files were pushed or pulled"
    );
}

struct ProjectionRemoteRequest {
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
    repo: Option<String>,
    locator: String,
}

fn request_from_action(action: ProjectionRemoteAction) -> ProjectionRemoteRequest {
    match action {
        ProjectionRemoteAction::Webdav { action } => {
            direction_request(RemoteProjectionProvider::WebDav, action)
        }
        ProjectionRemoteAction::S3 { action } => {
            direction_request(RemoteProjectionProvider::S3, action)
        }
    }
}

fn direction_request(
    provider: RemoteProjectionProvider,
    action: ProjectionRemoteDirectionAction,
) -> ProjectionRemoteRequest {
    match action {
        ProjectionRemoteDirectionAction::Push { repo, locator } => ProjectionRemoteRequest {
            provider,
            direction: RemoteProjectionDirection::Push,
            repo,
            locator,
        },
        ProjectionRemoteDirectionAction::Pull { repo, locator } => ProjectionRemoteRequest {
            provider,
            direction: RemoteProjectionDirection::Pull,
            repo,
            locator,
        },
    }
}

fn provider_direction_wired_for(request: &ProjectionRemoteRequest) -> bool {
    matches!(
        (request.provider, request.direction),
        (
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Push
        )
    )
}

#[cfg(test)]
mod tests;
