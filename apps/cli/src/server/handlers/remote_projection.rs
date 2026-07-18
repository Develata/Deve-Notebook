//! plan_ref:
//!   - 07_network#remote-import-wire-contract
//!   - 06_backup#remote-projection-transport-contract
//!
//! Remote Projection push use case. It is deliberately independent from
//! Source Control and Remote Import authority.

use crate::remote_projection_transport::{self, TransportCapability};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use deve_core::protocol::{
    RemoteProjectionPushRequest, RemoteProjectionPushResponse, ServerError, ServerErrorCode,
    ServerMessage,
};
use std::sync::Arc;

pub async fn handle_push(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    request: RemoteProjectionPushRequest,
) {
    let scope =
        match crate::server::repo_scope::resolve_session_repo_or_bootstrap_local(state, session) {
            Ok(scope)
                if scope.repo_id == request.repo_id
                    && scope.branch == request.branch
                    && scope.branch.is_none()
                    && session.scope_nonce() == request.scope_nonce.get() =>
            {
                scope
            }
            Ok(_) => return send_error(ch, &request, ServerErrorCode::RemoteProjectionPushFailed),
            Err(error) => {
                tracing::warn!(%error, "Remote Projection push scope resolution failed");
                return send_error(ch, &request, ServerErrorCode::RemoteProjectionPushFailed);
            }
        };
    if let Err(error) = super::local_writer::require_exact_local_writer(state, session, &scope) {
        tracing::warn!(code = ?error.code, detail = ?error.detail, "Remote Projection push writer admission failed");
        return send_response(ch, &request, Some(ServerError::new(error.code)));
    }
    let repo_url = match state.repo.get_repo_url(None, &scope.repo_name) {
        Ok(Some(locator)) => locator,
        Ok(None) => {
            return send_error(
                ch,
                &request,
                ServerErrorCode::RemoteProjectionLocatorInvalid,
            );
        }
        Err(error) => {
            tracing::warn!(%error, "Remote Projection push locator lookup failed");
            return send_error(
                ch,
                &request,
                ServerErrorCode::RemoteProjectionLocatorInvalid,
            );
        }
    };
    let locator = match remote_projection_transport::admit_repo_url(
        request.provider,
        TransportCapability::Push,
        &repo_url,
    ) {
        Ok(locator) => locator,
        Err(error) => {
            tracing::warn!(%error, "Remote Projection push locator admission failed");
            return send_error(
                ch,
                &request,
                ServerErrorCode::RemoteProjectionLocatorInvalid,
            );
        }
    };

    let repo = state.repo.clone();
    let repo_name = scope.repo_name;
    let provider = request.provider;
    let ledger_dir = repo.ledger_dir().to_path_buf();
    let result = state
        .repo_mutation_gate()
        .execute_mounted_repo_unpublished_blocking(request.repo_id, move || {
            crate::workspace_identity_gate::ensure_local_repo_workspace_identity_for_write(
                repo.as_ref(),
                &repo_name,
                "remote projection push",
            )
            .map_err(PushFailure::Workspace)?;
            let workspace = repo
                .local_repo_workspace_root(&repo_name)
                .map_err(PushFailure::Workspace)?;
            let source =
                remote_projection_transport::WorkspaceProjectionPushSource::collect(&workspace)
                    .map_err(PushFailure::Workspace)?;
            let outcome = remote_projection_transport::push_projection_from_source(
                &ledger_dir,
                provider,
                &locator,
                &source,
            )
            .map_err(PushFailure::Transport)?;
            Ok::<_, PushFailure>(outcome.uploaded_files)
        })
        .await;

    match result {
        Ok(Ok(uploaded_files)) => {
            tracing::info!(
                provider = request.provider.as_str(),
                uploaded_files,
                "Remote Projection push completed"
            );
            send_response(ch, &request, None);
        }
        Ok(Err(PushFailure::Workspace(error))) => {
            tracing::warn!(%error, "Remote Projection push source capture failed");
            send_error(ch, &request, ServerErrorCode::RemoteProjectionPushFailed);
        }
        Ok(Err(PushFailure::Transport(error))) => {
            let code = if error.is_provider_unavailable() {
                ServerErrorCode::RemoteProjectionProviderUnavailable
            } else {
                ServerErrorCode::RemoteProjectionPushFailed
            };
            tracing::warn!(%error, ?code, "Remote Projection transport push failed");
            send_error(ch, &request, code);
        }
        Err(error) => {
            tracing::warn!(%error, "Remote Projection push mount gate failed");
            let error = if error
                == crate::server::repo_mutation::RepoMutationGateError::WorkspaceIngestionUnavailable
            {
                ServerError::workspace_ingestion_unavailable()
            } else {
                ServerError::new(ServerErrorCode::RemoteProjectionPushFailed)
            };
            send_response(ch, &request, Some(error));
        }
    }
}

enum PushFailure {
    Workspace(anyhow::Error),
    Transport(remote_projection_transport::ProjectionPushError),
}

fn send_error(ch: &DualChannel, request: &RemoteProjectionPushRequest, code: ServerErrorCode) {
    send_response(ch, request, Some(ServerError::new(code)));
}

fn send_response(
    ch: &DualChannel,
    request: &RemoteProjectionPushRequest,
    error: Option<ServerError>,
) {
    ch.unicast(ServerMessage::RemoteProjectionPush(
        RemoteProjectionPushResponse {
            request_id: request.request_id,
            repo_id: request.repo_id,
            branch: request.branch.clone(),
            scope_nonce: request.scope_nonce,
            error,
        },
    ));
}
