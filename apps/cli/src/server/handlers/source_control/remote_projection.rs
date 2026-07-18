//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#command-palette-shortcuts
//!
//! WebSocket execution for remote Markdown projection transport intents.

use crate::remote_projection_legacy::{self, PreparedProjectionRemotePull};
use crate::remote_projection_transport::{self, TransportCapability};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use anyhow::Result;
use deve_core::protocol::{
    REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL, RemoteProjectionDirection,
    RemoteProjectionProvider,
};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RemoteProjectionExecutionSummary {
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
    provider_io_ready: bool,
    uploaded_files: usize,
    downloaded_files: usize,
    external_changes_scan_triggered: bool,
}

impl RemoteProjectionExecutionSummary {
    pub(super) fn from_legacy_pull(
        summary: remote_projection_legacy::LegacyPullExecutionSummary,
    ) -> Self {
        Self {
            provider: summary.provider,
            direction: RemoteProjectionDirection::Pull,
            provider_io_ready: true,
            uploaded_files: 0,
            downloaded_files: summary.downloaded_files,
            external_changes_scan_triggered: summary.external_changes_scan_triggered,
        }
    }

    fn from_push(provider: RemoteProjectionProvider, uploaded_files: usize) -> Self {
        Self {
            provider,
            direction: RemoteProjectionDirection::Push,
            provider_io_ready: true,
            uploaded_files,
            downloaded_files: 0,
            external_changes_scan_triggered: false,
        }
    }
}

pub async fn handle_remote_projection_transport(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
) {
    handle_remote_projection_transport_with_executors(
        state,
        ch,
        session,
        provider,
        direction,
        remote_projection_legacy::prepare_pull_for_resolved_repo,
        execute_push_for_resolved_repo,
    )
    .await;
}

async fn handle_remote_projection_transport_with_executors<F, P>(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
    pull_preparer: F,
    push_executor: P,
) where
    F: FnOnce(RemoteProjectionProvider, &str) -> Result<PreparedProjectionRemotePull>
        + Send
        + 'static,
    P: FnOnce(
            Arc<deve_core::ledger::RepoManager>,
            String,
            RemoteProjectionProvider,
            String,
        ) -> Result<RemoteProjectionExecutionSummary>
        + Send
        + 'static,
{
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope =
        match super::repo_scope::resolve_current_authorized_writable_local_repo(state, session) {
            Ok(scope) => scope,
            Err(error) => return super::errors::send_ws_scoped(ch, error, scope_nonce),
        };
    let locator =
        match remote_projection_locator_from_repo_url(state, &scope.repo_name, provider, direction)
        {
            Ok(locator) => locator,
            Err(error) => {
                return super::errors::send_ws_code_scoped(
                    ch,
                    ServerErrorCode::ScRepoContextInvalid,
                    remote_projection_provider_io_not_ready_detail(error),
                    scope_nonce,
                );
            }
        };

    let repo = state.repo.clone();
    let repo_name = scope.repo_name.clone();
    let repo_id = scope.repo_id;
    let result = if direction == RemoteProjectionDirection::Pull {
        let gate = state.repo_mutation_gate();
        let admission = match gate.admit_mounted_repo(repo_id) {
            Ok(admission) => admission,
            Err(error) => {
                return super::errors::send_ws_scoped(ch, error.server_error(), scope_nonce);
            }
        };
        super::remote_projection_pull::execute(
            super::remote_projection_pull::PullExecutionInput {
                state: state.clone(),
                gate,
                admission,
                repo_name: repo_name.clone(),
                repo_id,
                provider,
                locator,
            },
            pull_preparer,
        )
        .await
    } else {
        if let Err(error) = state.repo_mutation_gate().admit_mounted_repo(repo_id) {
            return super::errors::send_ws_scoped(ch, error.server_error(), scope_nonce);
        }
        match tokio::task::spawn_blocking(move || push_executor(repo, repo_name, provider, locator))
            .await
        {
            Ok(Ok(summary)) => Ok(summary),
            Ok(Err(error)) => Err(ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                remote_projection_provider_io_not_ready_detail(error),
            )),
            Err(error) => Err(ServerError::with_detail(
                ServerErrorCode::ScRepoContextInvalid,
                remote_projection_provider_io_not_ready_detail(error),
            )),
        }
    };
    match result {
        Ok(summary) => {
            tracing::info!(
                provider = summary.provider.as_str(),
                direction = summary.direction.as_str(),
                repo = scope.repo_name,
                provider_io_ready = summary.provider_io_ready,
                uploaded_files = summary.uploaded_files,
                downloaded_files = summary.downloaded_files,
                external_changes_scan_triggered = summary.external_changes_scan_triggered,
                "remote projection provider I/O completed"
            );
            super::changes::handle_get_changes(state, ch, session, None).await;
        }
        Err(error) => super::errors::send_ws_scoped(ch, error, scope_nonce),
    }
}

fn execute_push_for_resolved_repo(
    repo: Arc<deve_core::ledger::RepoManager>,
    repo_name: String,
    provider: RemoteProjectionProvider,
    locator: String,
) -> Result<RemoteProjectionExecutionSummary> {
    crate::workspace_identity_gate::ensure_local_repo_workspace_identity_for_write(
        repo.as_ref(),
        &repo_name,
        "remote projection transport",
    )?;
    let workspace = repo.local_repo_workspace_root(&repo_name)?;
    let source = remote_projection_transport::WorkspaceProjectionPushSource::collect(&workspace)?;
    let outcome =
        remote_projection_transport::push_projection_from_source(provider, &locator, &source)?;
    Ok(RemoteProjectionExecutionSummary::from_push(
        provider,
        outcome.uploaded_files,
    ))
}

fn remote_projection_locator_from_repo_url(
    state: &Arc<AppState>,
    repo_name: &str,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
) -> Result<String> {
    let repo_url = state.repo.get_repo_url(None, repo_name)?.ok_or_else(|| {
        anyhow::anyhow!("remote projection locator is not configured on repo_url")
    })?;
    let capability = match direction {
        RemoteProjectionDirection::Push => TransportCapability::Push,
        RemoteProjectionDirection::Pull => TransportCapability::SourceAcquisition,
    };
    remote_projection_transport::admit_repo_url_without_profile(provider, capability, &repo_url)
        .map_err(Into::into)
}

pub(super) fn remote_projection_provider_io_not_ready_detail(
    error: impl std::fmt::Display,
) -> String {
    format!("{REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL}; {error}")
}

#[cfg(test)]
mod tests;
