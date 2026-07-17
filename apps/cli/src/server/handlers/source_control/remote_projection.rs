//! plan_ref:
//!   - 05_diff_logic#remote-projection-transport
//!   - 14_commands#command-palette-shortcuts
//!
//! WebSocket execution for remote Markdown projection transport intents.

use crate::commands::projection_remote::{self, PreparedProjectionRemotePull};
use crate::server::{AppState, channel::DualChannel, session::WsSession};
use anyhow::Result;
use deve_core::protocol::{
    REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL, RemoteProjectionDirection,
    RemoteProjectionProvider,
};
use deve_core::protocol::{ServerError, ServerErrorCode};
use std::sync::Arc;

pub async fn handle_remote_projection_transport(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
) {
    handle_remote_projection_transport_with_pull_preparer(
        state,
        ch,
        session,
        provider,
        direction,
        projection_remote::prepare_pull_for_resolved_repo,
    )
    .await;
}

async fn handle_remote_projection_transport_with_pull_preparer<F>(
    state: &Arc<AppState>,
    ch: &DualChannel,
    session: &mut WsSession,
    provider: RemoteProjectionProvider,
    direction: RemoteProjectionDirection,
    pull_preparer: F,
) where
    F: FnOnce(RemoteProjectionProvider, &str) -> Result<PreparedProjectionRemotePull>
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
        match tokio::task::spawn_blocking(move || {
            projection_remote::run_for_resolved_repo(
                repo, &repo_name, provider, direction, &locator,
            )
        })
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

fn remote_projection_locator_from_repo_url(
    state: &Arc<AppState>,
    repo_name: &str,
    provider: RemoteProjectionProvider,
) -> Result<String> {
    let repo_url = state.repo.get_repo_url(None, repo_name)?.ok_or_else(|| {
        anyhow::anyhow!("remote projection locator is not configured on repo_url")
    })?;
    ensure_repo_url_provider_io_supported(provider, &repo_url)?;
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

fn ensure_repo_url_provider_io_supported(
    provider: RemoteProjectionProvider,
    locator: &str,
) -> Result<()> {
    if provider == RemoteProjectionProvider::S3 && is_s3_custom_https_locator(locator) {
        anyhow::bail!(
            "S3 custom endpoint requires explicit credential binding before Web Remote Projection provider I/O"
        );
    }
    Ok(())
}

fn is_s3_custom_https_locator(locator: &str) -> bool {
    locator
        .trim()
        .get(..11)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("s3+https://"))
}

pub(super) fn remote_projection_provider_io_not_ready_detail(
    error: impl std::fmt::Display,
) -> String {
    format!("{REMOTE_PROJECTION_PROVIDER_IO_PENDING_DETAIL}; {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::runtime::watcher_runtime::{RepoMountState, WatcherRuntimeView};
    use crate::server::sync_hello_test_support::{build_state, unicast_channel};
    use deve_core::protocol::ServerMessage;
    use deve_core::remote_projection::RemoteProjectionFile;
    use deve_core::source_control::ChangeStatus;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::time::{Duration, timeout};

    #[test]
    fn s3_custom_endpoint_repo_url_gate_is_case_insensitive() {
        let error = ensure_repo_url_provider_io_supported(
            RemoteProjectionProvider::S3,
            " S3+HTTPS://minio.example.com/bucket/notebooks/main",
        )
        .expect_err("mixed-case s3 custom endpoint must fail closed");

        assert!(error.to_string().contains("explicit credential binding"));
    }

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
    async fn workspace_ingestion_error_mapping_remote_projection_uses_protocol_error()
    -> anyhow::Result<()> {
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
        let repo_id = state
            .repo
            .get_repo_info_for(None, Some("notes"))?
            .expect("notes repo")
            .uuid;
        state.set_watcher_runtime_view_for_test(WatcherRuntimeView::with_state_for_test(
            repo_id,
            1,
            RepoMountState::Mounted,
        ));
        state.repo.ensure_local_repo_workspace_identity("notes")?;
        let (ch, mut uni_rx) = unicast_channel(&state);
        let mut session = crate::server::session::WsSession::new();

        handle_remote_projection_transport_with_pull_preparer(
            &state,
            &ch,
            &mut session,
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Pull,
            |provider, locator| {
                assert_eq!(provider, RemoteProjectionProvider::WebDav);
                assert_eq!(locator, "webdav+https://dav.example.com/notebooks/main");
                Ok(projection_remote::prepared_pull_for_test(
                    provider,
                    vec![RemoteProjectionFile::new("remote.md", b"remote".to_vec())?],
                ))
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

        state.set_watcher_runtime_view_for_test(WatcherRuntimeView::with_state_for_test(
            repo_id,
            1,
            RepoMountState::Failed,
        ));
        let executor_called = Arc::new(AtomicBool::new(false));
        let observed = executor_called.clone();
        handle_remote_projection_transport_with_pull_preparer(
            &state,
            &ch,
            &mut session,
            RemoteProjectionProvider::WebDav,
            RemoteProjectionDirection::Pull,
            move |provider, _| {
                observed.store(true, Ordering::SeqCst);
                Ok(projection_remote::prepared_pull_for_test(
                    provider,
                    Vec::new(),
                ))
            },
        )
        .await;
        assert!(!executor_called.load(Ordering::SeqCst));
        match timeout(Duration::from_secs(2), uni_rx.recv())
            .await?
            .expect("mount blocker")
        {
            ServerMessage::ProtocolError { error, .. } => assert_eq!(
                error.code,
                ServerErrorCode::StorageWorkspaceIngestionUnavailable
            ),
            other => panic!("expected mount ProtocolError, got {other:?}"),
        }
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn remote_projection_transport_rejects_s3_custom_endpoint_repo_url_before_executor()
    -> anyhow::Result<()> {
        for repo_url in [
            "s3+https://minio.example.com/bucket/notebooks/main",
            "S3+HTTPS://minio.example.com/bucket/notebooks/main",
        ] {
            let dir = tempfile::tempdir()?;
            let ledger_dir = dir.path().join("ledger");
            let projection_base = dir.path().join("notes");
            let mut repo = deve_core::ledger::RepoManager::init(
                &ledger_dir,
                10,
                Some("notes"),
                Some(repo_url),
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
            let executor_called = Arc::new(AtomicBool::new(false));
            let executor_called_for_closure = executor_called.clone();

            handle_remote_projection_transport_with_pull_preparer(
                &state,
                &ch,
                &mut session,
                RemoteProjectionProvider::S3,
                RemoteProjectionDirection::Pull,
                move |provider, _locator| {
                    executor_called_for_closure.store(true, Ordering::SeqCst);
                    Ok(projection_remote::prepared_pull_for_test(
                        provider,
                        Vec::new(),
                    ))
                },
            )
            .await;

            assert!(!executor_called.load(Ordering::SeqCst), "{repo_url}");
            match timeout(Duration::from_secs(2), uni_rx.recv())
                .await?
                .expect("protocol error")
            {
                ServerMessage::ProtocolError {
                    error, scope_nonce, ..
                } => {
                    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
                    assert_eq!(scope_nonce, None);
                    let detail = error.detail.as_deref().expect("detail");
                    assert!(detail.contains("provider_io_ready=false"), "{detail}");
                    assert!(detail.contains("explicit credential binding"), "{detail}");
                }
                other => panic!("expected ProtocolError for {repo_url}, got {other:?}"),
            }
        }
        Ok(())
    }
}
