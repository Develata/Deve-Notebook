//! plan_ref:
//!   - 06_backup#projection-backup-command-output-contract
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#remote-import-command-contract
//!
//! Remote Import CLI shell. Domain work stays in the shared host coordinator.

mod direct;
mod output;
mod proxy;

use crate::commands::live_proxy;
use anyhow::Result;
use clap::{Args, Subcommand, ValueEnum};
use deve_core::ledger::RepoManager;
use deve_core::models::RepoId;
use deve_core::protocol::RemoteProjectionProvider;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Args, Debug)]
pub(crate) struct LocalCliAuthArgs {
    /// Operator username used only if the owner server holds the repo DB
    #[arg(long)]
    pub(crate) auth_user: Option<String>,
    /// Read the operator password from stdin only if proxy fallback is required
    #[arg(long)]
    pub(crate) auth_password_stdin: bool,
}

#[derive(Subcommand, Debug)]
pub(crate) enum RemoteImportAction {
    /// Capture one immutable remote source snapshot
    Prepare {
        #[arg(value_enum)]
        provider: RemoteImportProviderArg,
        #[arg(long)]
        repo: RepoId,
    },
    /// List durable sessions for one exact repo
    List {
        #[arg(long)]
        repo: RepoId,
    },
    /// Show one exact session, or its first candidate page when revision is supplied
    Show {
        #[arg(long)]
        repo: RepoId,
        #[arg(long)]
        session: Uuid,
        #[arg(long)]
        revision: Option<u64>,
    },
    /// Compute a typed diff for one opaque candidate entry
    Diff {
        #[arg(long)]
        repo: RepoId,
        #[arg(long)]
        session: Uuid,
        #[arg(long)]
        revision: u64,
        #[arg(long)]
        entry: String,
    },
    /// Recompute a candidate from already sealed blobs
    Refresh {
        #[arg(long)]
        repo: RepoId,
        #[arg(long)]
        session: Uuid,
        #[arg(long)]
        revision: u64,
    },
    /// Apply the entire candidate through the sealed Ledger writer
    Apply {
        #[arg(long)]
        repo: RepoId,
        #[arg(long)]
        session: Uuid,
        #[arg(long)]
        revision: u64,
        /// Stable identity for retry after a lost response
        #[arg(long)]
        request_id: Option<Uuid>,
    },
    /// Discard one exact session revision; omit revision only for a pre-candidate failure
    Discard {
        #[arg(long)]
        repo: RepoId,
        #[arg(long)]
        session: Uuid,
        #[arg(long)]
        revision: Option<u64>,
    },
    /// Inspect cleanup debt; actual cleanup requires --apply
    Repair {
        #[arg(long)]
        repo: RepoId,
        #[arg(long)]
        apply: bool,
    },
}

impl RemoteImportAction {
    pub(crate) fn repo_id(&self) -> RepoId {
        match self {
            Self::Prepare { repo, .. }
            | Self::List { repo }
            | Self::Show { repo, .. }
            | Self::Diff { repo, .. }
            | Self::Refresh { repo, .. }
            | Self::Apply { repo, .. }
            | Self::Discard { repo, .. }
            | Self::Repair { repo, .. } => *repo,
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub(crate) enum RemoteImportProviderArg {
    Webdav,
    S3,
}

impl From<RemoteImportProviderArg> for RemoteProjectionProvider {
    fn from(value: RemoteImportProviderArg) -> Self {
        match value {
            RemoteImportProviderArg::Webdav => Self::WebDav,
            RemoteImportProviderArg::S3 => Self::S3,
        }
    }
}

pub(crate) fn run(
    ledger_dir: &Path,
    action: RemoteImportAction,
    auth: LocalCliAuthArgs,
    snapshot_depth: usize,
) -> Result<()> {
    match RepoManager::init(ledger_dir, snapshot_depth, None, None) {
        Ok(repo) => direct::run(Arc::new(repo), action),
        Err(error) if live_proxy::is_db_lock_error(&error) => proxy::run(ledger_dir, action, auth),
        Err(error) => Err(error),
    }
}
