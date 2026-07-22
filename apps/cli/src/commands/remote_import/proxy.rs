//! plan_ref:
//!   - 08_auth#local-cli-proxy-authority
//!   - 14_commands#remote-import-command-contract

use super::{LocalCliAuthArgs, RemoteImportAction, output};
use crate::commands::live_proxy;
use crate::local_cli_proxy_contract::{LocalCliRemoteImportRequest, LocalCliRemoteImportResponse};
use anyhow::{Context, Result, anyhow, bail};
use deve_core::protocol::{
    RemoteImportCandidateRevision, RemoteImportEntryId, RemoteImportPageCursor,
    RemoteImportRequest, RemoteImportRequestContext, RemoteImportSessionId, ScopeNonce,
};
use std::path::Path;
use uuid::Uuid;

const LOCAL_CLI_SCOPE_NONCE: u64 = 1;

pub(super) fn run(
    ledger_dir: &Path,
    action: RemoteImportAction,
    auth: LocalCliAuthArgs,
) -> Result<()> {
    live_proxy::block_on_safe(async move {
        let session =
            live_proxy::authenticated_session(ledger_dir, auth.auth_user, auth.auth_password_stdin)
                .await?;
        let request = request_from_action(action);
        let response = session
            .post("/api/local-cli/remote-import")
            .json(&request)
            .send()
            .await
            .context("Local CLI Remote Import proxy request failed")?;
        let status = response.status();
        let bytes = response
            .bytes()
            .await
            .context("Local CLI Remote Import proxy response body failed")?;
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            bail!(live_proxy::decode_auth_rejection(&bytes)?);
        }
        let response = serde_json::from_slice::<LocalCliRemoteImportResponse>(&bytes)
            .map_err(|_| anyhow!("REMOTE_IMPORT_INVALID_STATE"))?;
        output::print(&response)?;
        output::ensure_success(&response)?;
        if !status.is_success() {
            bail!("REMOTE_IMPORT_INVALID_STATE");
        }
        Ok(())
    })
}

fn request_from_action(action: RemoteImportAction) -> LocalCliRemoteImportRequest {
    let repo_id = action.repo_id();
    let request_id = match &action {
        RemoteImportAction::Apply { request_id, .. } => {
            let request_id = request_id.unwrap_or_else(Uuid::new_v4);
            eprintln!("remote_import_apply_request_id={request_id}");
            request_id
        }
        _ => Uuid::new_v4(),
    };
    let context = RemoteImportRequestContext {
        request_id,
        repo_id,
        branch: None,
        scope_nonce: ScopeNonce::new(LOCAL_CLI_SCOPE_NONCE),
    };
    match action {
        RemoteImportAction::Prepare { provider, .. } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::Prepare {
                context,
                provider: provider.into(),
            },
        },
        RemoteImportAction::List { .. } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::List { context },
        },
        RemoteImportAction::Show {
            session, revision, ..
        } => LocalCliRemoteImportRequest::Intent {
            request: match revision {
                Some(revision) => RemoteImportRequest::Page {
                    context,
                    session_id: RemoteImportSessionId::new(session),
                    revision: RemoteImportCandidateRevision::new(revision),
                    cursor: None::<RemoteImportPageCursor>,
                    limit: deve_core::protocol::remote_import::REMOTE_IMPORT_DEFAULT_PAGE_SIZE,
                },
                None => RemoteImportRequest::Show {
                    context,
                    session_id: RemoteImportSessionId::new(session),
                    revision: None,
                },
            },
        },
        RemoteImportAction::Diff {
            session,
            revision,
            entry,
            ..
        } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::Diff {
                context,
                session_id: RemoteImportSessionId::new(session),
                revision: RemoteImportCandidateRevision::new(revision),
                entry_id: RemoteImportEntryId::new(entry),
            },
        },
        RemoteImportAction::Refresh {
            session, revision, ..
        } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::Refresh {
                context,
                session_id: RemoteImportSessionId::new(session),
                revision: RemoteImportCandidateRevision::new(revision),
            },
        },
        RemoteImportAction::Apply {
            session, revision, ..
        } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::Apply {
                context,
                session_id: RemoteImportSessionId::new(session),
                revision: RemoteImportCandidateRevision::new(revision),
            },
        },
        RemoteImportAction::Discard {
            session, revision, ..
        } => LocalCliRemoteImportRequest::Intent {
            request: RemoteImportRequest::Discard {
                context,
                session_id: RemoteImportSessionId::new(session),
                revision: revision.map(RemoteImportCandidateRevision::new),
            },
        },
        RemoteImportAction::Repair { apply, .. } => {
            LocalCliRemoteImportRequest::Repair { context, apply }
        }
    }
}
