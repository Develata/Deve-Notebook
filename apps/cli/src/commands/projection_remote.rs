//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#cli-commands
//!
//! CLI shell for remote Markdown projection transport intents.

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
    let request = request_from_action(action);
    let plan = plan_remote_projection_transport(RemoteProjectionPlanInput {
        provider: request.provider,
        direction: request.direction,
        locator: request.locator,
    })?;

    let repo = RepoManager::init(ledger_dir, snapshot_depth, None, None)?;
    let repo_names = resolve_local_repo_args(&repo, request.repo.as_deref())?;
    for repo_name in repo_names {
        ensure_local_repo_workspace_identity_for_write(
            &repo,
            &repo_name,
            "remote projection transport",
        )?;
        let workspace = repo.local_repo_workspace_root(&repo_name)?;
        println!(
            "projection_remote[{repo_name}]: provider={} direction={} scope={} workspace={} writes_ledger={} external_changes_confirmation_required={} provider_io_ready={}",
            plan.provider.as_str(),
            plan.direction.as_str(),
            plan.projection_scope,
            workspace.display(),
            plan.writes_ledger,
            plan.external_changes_confirmation_required,
            plan.provider_io_ready,
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn webdav_push_builds_provider_request() {
        let request = request_from_action(ProjectionRemoteAction::Webdav {
            action: ProjectionRemoteDirectionAction::Push {
                repo: Some("default".into()),
                locator: "webdav+https://dav.example.com/notebooks/main".into(),
            },
        });

        assert_eq!(request.provider, RemoteProjectionProvider::WebDav);
        assert_eq!(request.direction, RemoteProjectionDirection::Push);
        assert_eq!(request.repo.as_deref(), Some("default"));
    }

    #[test]
    fn s3_pull_builds_provider_request() {
        let request = request_from_action(ProjectionRemoteAction::S3 {
            action: ProjectionRemoteDirectionAction::Pull {
                repo: None,
                locator: "s3://bucket/notebooks/main".into(),
            },
        });

        assert_eq!(request.provider, RemoteProjectionProvider::S3);
        assert_eq!(request.direction, RemoteProjectionDirection::Pull);
        assert_eq!(request.locator, "s3://bucket/notebooks/main");
    }

    #[test]
    fn run_reports_provider_io_fail_closed_after_workspace_gate() {
        let repo = initialized_default_repo();

        let err = run(&repo.ledger_dir(), webdav_pull_action(), 8)
            .expect_err("provider I/O must remain fail-closed");

        let message = err.to_string();
        assert!(message.contains("provider I/O is not wired yet"));
        assert!(message.contains("provider_io_ready=false"));
    }

    #[test]
    fn run_checks_workspace_identity_before_provider_io() {
        let repo = initialized_default_repo();
        std::fs::remove_file(deve_core::utils::notegit::repo_identity_path(
            &repo.workspace,
        ))
        .expect("remove identity marker");

        let err = run(&repo.ledger_dir(), webdav_pull_action(), 8)
            .expect_err("workspace identity gate must fail before provider I/O");

        let message = err.to_string();
        assert!(message.contains("Projection workspace identity marker is invalid"));
        assert!(message.contains("identity marker"));
        assert!(!message.contains("provider_io_ready=false"));
    }

    struct ProjectionRemoteHarness {
        _dir: tempfile::TempDir,
        root: PathBuf,
        workspace: PathBuf,
    }

    impl ProjectionRemoteHarness {
        fn ledger_dir(&self) -> PathBuf {
            self.root.join("ledger")
        }
    }

    fn initialized_default_repo() -> ProjectionRemoteHarness {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        crate::commands::init::run(
            &root.join("ledger"),
            "default",
            &root.join("notes"),
            root.clone(),
            8,
            None,
            None,
        )
        .expect("init");
        let workspace = std::fs::read_dir(root.join("notes"))
            .expect("notes dir")
            .map(|entry| entry.expect("workspace entry").path())
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("default--"))
            })
            .expect("default workspace");

        ProjectionRemoteHarness {
            _dir: dir,
            root,
            workspace,
        }
    }

    fn webdav_pull_action() -> ProjectionRemoteAction {
        ProjectionRemoteAction::Webdav {
            action: ProjectionRemoteDirectionAction::Pull {
                repo: Some("default".into()),
                locator: "webdav+https://dav.example.com/notebooks/main".into(),
            },
        }
    }
}
