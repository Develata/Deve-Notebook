use super::handlers::listing::handle_list_repos;
use super::{
    channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry, AppState,
};
use deve_core::config::SyncMode;
use deve_core::models::PeerId;
use deve_core::protocol::{ServerErrorCode, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let (repo, _repo_id) = crate::server::catalog_repo_support::catalog_initial_repo(
        &ledger,
        "default",
        &projection_base,
        10,
        Some("urn:default"),
    )?;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new_checked(repo.clone())?),
            tx,
            plugins: vec![],
            sync_engine: Arc::new(RepoScopedSyncEngine::new(
                PeerId::new("test-peer"),
                repo,
                SyncMode::Auto,
            )),
            tree_manager: Arc::new(RepoTreeRegistry::new()),
            #[cfg(feature = "search")]
            search_available: false,
            identity_key,
        }),
    ))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_repos_on_missing_shadow_branch_without_repo_hint_marks_scope_invalid_and_clears_session(
) -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(1));
    session.switch_branch(Some("missing-shadow".into()));

    handle_list_repos(&state, &ch, &mut session, Some("req-missing-shadow".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(error
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("Remote branch not available:")));
        }
        other => panic!("expected scoped ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn list_repos_on_missing_shadow_branch_with_repo_hint_clears_session() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(2));
    session.switch_branch(Some("missing-shadow".into()));
    session.switch_repo("ghost".into(), None);

    handle_list_repos(
        &state,
        &ch,
        &mut session,
        Some("req-missing-shadow-hint".into()),
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
            assert!(error
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("Remote branch not available:")));
        }
        other => panic!("expected scoped ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_branch, None);
    assert!(session.active_repo.is_none());
    assert!(session.active_repo_id.is_none());
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
