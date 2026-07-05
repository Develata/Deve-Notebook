//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#command-palette-shortcuts
//!
//! WebSocket execution for remote Markdown projection transport intents.

use crate::commands::projection_remote::{self, ProjectionRemoteExecutionSummary};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use anyhow::Result;
use deve_core::protocol::ServerErrorCode;
use deve_core::protocol::{
    REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL, RemoteProjectionDirection,
    RemoteProjectionProvider,
};
use std::sync::Arc;

pub async fn handle_remote_projection_transport(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
) {
    handle_remote_projection_transport_with_executor(
        state,
        ch,
        session,
        provider,
        direction,
        projection_remote::run_for_resolved_repo,
    )
    .await;
}

async fn handle_remote_projection_transport_with_executor<F>(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
    executor: F,
) where
    F: FnOnce(
            Arc<deve_core::ledger::RepoManager>,
            &str,
            RemoteProjectionProvider,
            RemoteProjectionDirection,
            &str,
        ) -> Result<ProjectionRemoteExecutionSummary>
        + Send
        + 'static,
{
    let scope_nonce = session.is_browser_session().then(|| session.scope_nonce());
    let scope =
        match super::repo_scope::resolve_current_authorized_writable_local_repo(state, session) {
            Ok(scope) => scope,
            Err(error) => return super::errors::send_ws_scoped(ch, error, scope_nonce),
        };
    let locator = match remote_projection_locator_from_repo_url(state, &scope.repo_name, provider) {
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
    let result = tokio::task::spawn_blocking(move || {
        executor(repo, &repo_name, provider, direction, &locator)
    })
    .await;
    match result {
        Ok(Ok(summary)) => {
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
        Ok(Err(error)) => super::errors::send_ws_code_scoped(
            ch,
            ServerErrorCode::ScRepoContextInvalid,
            remote_projection_provider_io_not_ready_detail(error),
            scope_nonce,
        ),
        Err(error) => super::errors::send_ws_code_scoped(
            ch,
            ServerErrorCode::ScRepoContextInvalid,
            remote_projection_provider_io_not_ready_detail(error),
            scope_nonce,
        ),
    }
}

fn remote_projection_locator_from_repo_url(
    state: &Arc<AppState>,
    repo_name: &str,
    provider: RemoteProjectionProvider,
) -> Result<String> {
    let repo_url = state.repo.get_repo_url(None, repo_name)?.ok_or_else(|| {
        anyhow::anyhow!("remote projection locator is not configured on repo_url")
    })?;
    deve_core::remote_projection::plan_remote_projection_transport(
        deve_core::remote_projection::RemoteProjectionPlanInput {
            provider,
            direction: RemoteProjectionDirection::Push,
            locator: repo_url,
        },
    )
    .map(|plan| plan.locator)
    .map_err(Into::into)
}

fn remote_projection_provider_io_not_ready_detail(error: impl std::fmt::Display) -> String {
    format!("{REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL}; {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::sync_hello_test_support::{build_state, unicast_channel};
    use deve_core::protocol::ServerMessage;
    use deve_core::source_control::ChangeStatus;
    use tokio::time::{Duration, timeout};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_projection_transport_missing_transport_url_fails_closed() -> anyhow::Result<()>
    {
        let (_dir, state, _repo_id) = build_state()?;
        state.repo.ensure_local_repo_workspace_identity("notes")?;
        let (ch, mut uni_rx) = unicast_channel(&state);
        let mut session = crate::server::session::WsSession::new();

        handle_remote_projection_transport(
            &state,
            &ch,
            &mut session,
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Pull,
        )
        .await;

        match timeout(Duration::from_secs(2), uni_rx.recv())
            .await?
            .expect("protocol error")
        {
            ServerMessage::ProtocolError {
                error, scope_nonce, ..
            } => {
                assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
                assert_eq!(scope_nonce, None);
                assert!(
                    error
                        .detail
                        .as_deref()
                        .expect("detail")
                        .contains("provider_io_ready=false")
                );
            }
            other => panic!("expected ProtocolError, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_projection_transport_uses_repo_url_locator() -> anyhow::Result<()> {
        let dir = tempfile::tempdir()?;
        let ledger_dir = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        let mut repo = deve_core::ledger::RepoManager::init(
            &ledger_dir,
            10,
            Some("notes"),
            Some("webdav+https://dav.example.com/notebooks/main"),
        )?;
        repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
        let repo = Arc::new(repo);
        let (tx, _rx) = tokio::sync::broadcast::channel(16);
        let identity_key =
            crate::server::security::load_or_generate_identity_key(&dir.path().join("host"))?;
        let state = Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(deve_core::sync::repo_scoped::RepoScopedSyncEngine::new(
                identity_key.peer_id(),
                repo,
                deve_core::config::SyncMode::Auto,
            )),
            tree_manager: Arc::new(crate::server::tree_state::RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key,
        });
        state.repo.ensure_local_repo_workspace_identity("notes")?;
        let (ch, mut uni_rx) = unicast_channel(&state);
        let mut session = crate::server::session::WsSession::new();

        handle_remote_projection_transport_with_executor(
            &state,
            &ch,
            &mut session,
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Pull,
            |repo, repo_name, provider, direction, locator| {
                assert_eq!(provider, RemoteProjectionProvider::WebDav);
                assert_eq!(direction, RemoteProjectionDirection::Pull);
                assert_eq!(locator, "webdav+https://dav.example.com/notebooks/main");
                let workspace = repo.local_repo_workspace_root(repo_name)?;
                std::fs::write(workspace.join("remote.md"), "remote")?;
                let sync_manager = deve_core::sync::SyncManager::new_checked(repo)?;
                sync_manager.scan_repo(repo_name)?;
                Ok(ProjectionRemoteExecutionSummary {
                    provider,
                    direction,
                    provider_io_ready: true,
                    uploaded_files: 0,
                    downloaded_files: 1,
                    external_changes_scan_triggered: true,
                })
            },
        )
        .await;

        match timeout(Duration::from_secs(2), uni_rx.recv())
            .await?
            .expect("changes")
        {
            ServerMessage::ChangesList { unstaged, .. } => {
                assert_eq!(unstaged.len(), 1);
                assert_eq!(unstaged[0].path, "remote.md");
                assert_eq!(unstaged[0].status, ChangeStatus::Added);
            }
            other => panic!("expected ChangesList, got {other:?}"),
        }
        Ok(())
    }
}
