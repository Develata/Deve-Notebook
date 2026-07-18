//! plan_ref:
//!   - 03_storage/watcher#watcher-contract
//!   - 06_backup#projection-backup-upload-state-machine-contract

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
    let error = remote_projection_transport::admit_repo_url_without_profile(
        RemoteProjectionProvider::S3,
        TransportCapability::Push,
        " S3+HTTPS://minio.example.com/bucket/notebooks/main",
    )
    .expect_err("mixed-case s3 custom endpoint must fail closed");

    assert!(error.to_string().contains("explicit credential binding"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_projection_transport_missing_transport_url_fails_closed() -> anyhow::Result<()> {
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

    handle_remote_projection_transport_with_executors(
        &state,
        &ch,
        &mut session,
        RemoteProjectionProvider::WebDav,
        RemoteProjectionDirection::Pull,
        |provider, locator| {
            assert_eq!(provider, RemoteProjectionProvider::WebDav);
            assert_eq!(locator, "webdav+https://dav.example.com/notebooks/main");
            Ok(remote_projection_legacy::prepared_pull_for_test(
                provider,
                vec![RemoteProjectionFile::new("remote.md", b"remote".to_vec())?],
            ))
        },
        execute_push_for_resolved_repo,
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

    let push_called = Arc::new(AtomicBool::new(false));
    let observed = push_called.clone();
    let expected_push_repo = state.repo.clone();
    let expected_push_repo_id = repo_id;
    handle_remote_projection_transport_with_executors(
        &state,
        &ch,
        &mut session,
        RemoteProjectionProvider::WebDav,
        RemoteProjectionDirection::Push,
        remote_projection_legacy::prepare_pull_for_resolved_repo,
        move |push_repo, repo_name, provider, locator| {
            observed.store(true, Ordering::SeqCst);
            assert!(Arc::ptr_eq(&push_repo, &expected_push_repo));
            assert_eq!(
                push_repo
                    .get_repo_info_for(None, Some(&repo_name))?
                    .expect("push repo")
                    .uuid,
                expected_push_repo_id
            );
            assert_eq!(provider, RemoteProjectionProvider::WebDav);
            assert_eq!(locator, "webdav+https://dav.example.com/notebooks/main");
            Ok(RemoteProjectionExecutionSummary::from_push(provider, 1))
        },
    )
    .await;
    assert!(push_called.load(Ordering::SeqCst));
    match timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("push changes refresh")
    {
        ServerMessage::ChangesList { unstaged, .. } => {
            assert_eq!(unstaged.len(), 1);
            assert_eq!(unstaged[0].path, "remote.md");
        }
        other => panic!("expected ChangesList after push, got {other:?}"),
    }

    handle_remote_projection_transport_with_executors(
        &state,
        &ch,
        &mut session,
        RemoteProjectionProvider::WebDav,
        RemoteProjectionDirection::Push,
        remote_projection_legacy::prepare_pull_for_resolved_repo,
        |_repo, _repo_name, _provider, _locator| Err(anyhow::anyhow!("injected provider failure")),
    )
    .await;
    match timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("push provider failure")
    {
        ServerMessage::ProtocolError { error, .. } => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(
                error
                    .detail
                    .as_deref()
                    .expect("detail")
                    .contains("provider_io_ready=false")
            );
        }
        other => panic!("expected push ProtocolError, got {other:?}"),
    }

    state.set_watcher_runtime_view_for_test(WatcherRuntimeView::with_state_for_test(
        repo_id,
        1,
        RepoMountState::Failed,
    ));
    let executor_called = Arc::new(AtomicBool::new(false));
    let observed = executor_called.clone();
    handle_remote_projection_transport_with_executors(
        &state,
        &ch,
        &mut session,
        RemoteProjectionProvider::WebDav,
        RemoteProjectionDirection::Pull,
        move |provider, _| {
            observed.store(true, Ordering::SeqCst);
            Ok(remote_projection_legacy::prepared_pull_for_test(
                provider,
                Vec::new(),
            ))
        },
        execute_push_for_resolved_repo,
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

    handle_remote_projection_transport(
        &state,
        &ch,
        &mut session,
        RemoteProjectionProvider::WebDav,
        RemoteProjectionDirection::Push,
    )
    .await;
    match timeout(Duration::from_secs(2), uni_rx.recv())
        .await?
        .expect("push mount blocker")
    {
        ServerMessage::ProtocolError { error, .. } => assert_eq!(
            error.code,
            ServerErrorCode::StorageWorkspaceIngestionUnavailable
        ),
        other => panic!("expected push mount ProtocolError, got {other:?}"),
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
        let mut repo =
            deve_core::ledger::RepoManager::init(&ledger_dir, 10, Some("notes"), Some(repo_url))?;
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

        handle_remote_projection_transport_with_executors(
            &state,
            &ch,
            &mut session,
            RemoteProjectionProvider::S3,
            RemoteProjectionDirection::Pull,
            move |provider, _locator| {
                executor_called_for_closure.store(true, Ordering::SeqCst);
                Ok(remote_projection_legacy::prepared_pull_for_test(
                    provider,
                    Vec::new(),
                ))
            },
            execute_push_for_resolved_repo,
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
