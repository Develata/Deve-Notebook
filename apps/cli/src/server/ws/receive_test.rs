use super::{SocketFlow, browser_scope_nonce, handle_incoming_message, invalid_client_message};
use crate::server::channel::DualChannel;
use crate::server::security;
use crate::server::tree_state::RepoTreeRegistry;
use crate::server::ws::filter::BroadcastFilter;
use crate::server::{AppState, session::WsSession};
use axum::extract::ws::Message;
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

#[test]
fn invalid_client_messages_use_structured_request_failed() {
    let error = invalid_client_message("Invalid JSON client message");
    assert_eq!(error.code, ServerErrorCode::RequestFailed);
    assert_eq!(error.detail.as_deref(), Some("Invalid JSON client message"));
}

#[test]
fn browser_scope_nonce_prefers_sync_scope() {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(7));
    session.set_sync_scope_nonce(11);
    assert_eq!(browser_scope_nonce(&session), Some(11));
}

#[test]
fn browser_scope_nonce_falls_back_to_current_scope() {
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(7));
    assert_eq!(browser_scope_nonce(&session), Some(7));
}

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(8);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_service: None,
            identity_key,
        }),
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_invalid_json_carries_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let filter = BroadcastFilter::allow_all();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(17));

    let flow = handle_incoming_message(
        &state,
        &ch,
        &mut session,
        Message::Text("{not-json".into()),
        &filter,
        "peer-1",
    )
    .await;

    assert!(matches!(flow, SocketFlow::Continue));
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(scope_nonce, Some(17));
        }
        other => panic!("expected scoped ProtocolError, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_invalid_bincode_prefers_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let filter = BroadcastFilter::allow_all();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(17));
    session.set_sync_scope_nonce(23);

    let flow = handle_incoming_message(
        &state,
        &ch,
        &mut session,
        Message::Binary(vec![0, 1, 2]),
        &filter,
        "peer-1",
    )
    .await;

    assert!(matches!(flow, SocketFlow::Continue));
    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError {
            error, scope_nonce, ..
        }) => {
            assert_eq!(error.code, ServerErrorCode::RequestFailed);
            assert_eq!(scope_nonce, Some(23));
        }
        other => panic!("expected scoped ProtocolError, got {:?}", other),
    }
    Ok(())
}
