//! plan_ref:
//!   - 07_network#remote-import-wire-contract
//!   - 04_repository#repo-scope-runtime
//!
//! Remote Import route identity and typed-error acceptance coverage.

use super::route_remote_import;
use crate::server::{AppState, channel::DualChannel, security, session::WsSession};
use deve_core::config::SyncMode;
use deve_core::ledger::{RepoManager, init::RepoInitOptions};
use deve_core::models::{PeerId, RepoId};
use deve_core::protocol::{
    ClientMessage, RemoteImportCandidateRevision, RemoteImportRequest, RemoteImportRequestContext,
    RemoteImportResponse, RemoteImportResponseContext, RemoteImportSessionId, ScopeNonce,
    ServerErrorCode, ServerMessage,
};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, timeout};
use uuid::Uuid;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_import_route_rejects_stale_scope_before_runtime_dispatch() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session(repo_id, 7);
    let request = show_request(repo_id, None, 6, session_id(), revision());

    route_remote_import(&state, &ch, &mut session, request).await;

    match recv(&mut rx).await? {
        ServerMessage::ProtocolError {
            error, scope_nonce, ..
        } => {
            assert_eq!(error.code, ServerErrorCode::ScStaleScope);
            assert_eq!(scope_nonce, Some(6));
            assert!(
                error
                    .detail
                    .as_deref()
                    .expect("typed scope detail")
                    .contains("remote import scope nonce is stale")
            );
        }
        other => panic!("expected scoped ProtocolError, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "stale scope must stop dispatch");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_import_route_echoes_exact_identity_for_repo_and_branch_rejection()
-> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session(repo_id, 7);
    let session_id = session_id();
    let revision = revision();

    let wrong_repo = RepoId::new_v4();
    route_remote_import(
        &state,
        &ch,
        &mut session,
        show_request(wrong_repo, None, 7, session_id, revision),
    )
    .await;
    assert_remote_error(
        recv(&mut rx).await?,
        expected_context(wrong_repo, None, 7, session_id, revision),
        ServerErrorCode::RemoteImportInvalidState,
    );

    let remote_branch = Some(PeerId::new("remote-shadow"));
    route_remote_import(
        &state,
        &ch,
        &mut session,
        show_request(repo_id, remote_branch.clone(), 7, session_id, revision),
    )
    .await;
    assert_remote_error(
        recv(&mut rx).await?,
        expected_context(repo_id, remote_branch, 7, session_id, revision),
        ServerErrorCode::RemoteImportInvalidState,
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_import_route_preserves_session_revision_on_typed_not_found() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = browser_session(repo_id, 7);
    let session_id = session_id();
    let revision = revision();

    route_remote_import(
        &state,
        &ch,
        &mut session,
        show_request(repo_id, None, 7, session_id, revision),
    )
    .await;

    assert_remote_error(
        recv(&mut rx).await?,
        expected_context(repo_id, None, 7, session_id, revision),
        ServerErrorCode::RemoteImportNotFound,
    );
    Ok(())
}

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, RepoId)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let repo_id = RepoId::new_v4();
    let mut repo = RepoManager::init_with_options(
        &ledger,
        10,
        Some("notes"),
        RepoInitOptions {
            repo_id: Some(repo_id),
            repo_url: Some("webdav+https://dav.example.com/notebooks/main".to_string()),
        },
    )?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(
        &deve_core::utils::notegit::host_keys_dir(repo.ledger_dir()),
    )?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                identity_key.peer_id(),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(crate::server::tree_state::RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key,
        }),
        repo_id,
    ))
}

fn browser_session(repo_id: RepoId, scope_nonce: u64) -> WsSession {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo("notes".to_string(), Some(repo_id));
    session.set_scope_nonce(Some(scope_nonce));
    session
}

fn unicast_channel(state: &Arc<AppState>) -> (DualChannel, mpsc::Receiver<ServerMessage>) {
    let (tx, rx) = mpsc::channel(16);
    (DualChannel::new(state.tx.clone(), tx), rx)
}

fn show_request(
    repo_id: RepoId,
    branch: Option<PeerId>,
    scope_nonce: u64,
    session_id: RemoteImportSessionId,
    revision: RemoteImportCandidateRevision,
) -> ClientMessage {
    ClientMessage::RemoteImport(RemoteImportRequest::Show {
        context: RemoteImportRequestContext {
            request_id: request_id(),
            repo_id,
            branch,
            scope_nonce: ScopeNonce::new(scope_nonce),
        },
        session_id,
        revision: Some(revision),
    })
}

fn expected_context(
    repo_id: RepoId,
    branch: Option<PeerId>,
    scope_nonce: u64,
    session_id: RemoteImportSessionId,
    revision: RemoteImportCandidateRevision,
) -> RemoteImportResponseContext {
    RemoteImportResponseContext {
        request_id: request_id(),
        repo_id,
        branch,
        scope_nonce: ScopeNonce::new(scope_nonce),
        session_id: Some(session_id),
        revision: Some(revision),
    }
}

fn request_id() -> Uuid {
    Uuid::from_u128(0x100)
}

fn session_id() -> RemoteImportSessionId {
    RemoteImportSessionId::new(Uuid::from_u128(0x200))
}

fn revision() -> RemoteImportCandidateRevision {
    RemoteImportCandidateRevision::new(5)
}

async fn recv(rx: &mut mpsc::Receiver<ServerMessage>) -> anyhow::Result<ServerMessage> {
    Ok(timeout(Duration::from_secs(2), rx.recv())
        .await?
        .expect("Remote Import route response"))
}

fn assert_remote_error(
    message: ServerMessage,
    expected_context: RemoteImportResponseContext,
    expected_code: ServerErrorCode,
) {
    match message {
        ServerMessage::RemoteImport(RemoteImportResponse::Error { context, error }) => {
            assert_eq!(context, expected_context);
            assert_eq!(error.code, expected_code);
            assert!(
                error.detail.is_none(),
                "raw runtime detail must stay private"
            );
        }
        other => panic!("expected typed Remote Import error, got {other:?}"),
    }
}
