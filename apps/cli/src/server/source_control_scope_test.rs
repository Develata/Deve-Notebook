use super::handlers::source_control::{handle_get_changes, handle_get_commit_history};
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::models::PeerId;
use deve_core::protocol::{ScPathTarget, ServerMessage};
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, CommitInfo};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
    let default_id = repo.get_repo_info()?.expect("default info").uuid;
    let mut test_repo = RepoManager::init(dir.path(), 10, Some("test"), Some("urn:test"))?;
    test_repo.set_vault_root(&vault);
    let test_id = test_repo.get_repo_info()?.expect("test info").uuid;
    let repo = Arc::new(repo);
    let (tx, _rx) = broadcast::channel(16);
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
        default_id,
        test_id,
    ))
}

fn seed_pending(repo: &RepoManager, repo_name: &str, path: &str, content: &str) {
    repo.run_on_local_repo(repo_name, |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending");
}

fn write_workspace_file(dir: &TempDir, repo_name: &str, path: &str, content: &str) {
    let abs = dir.path().join("vault").join(repo_name).join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

async fn recv_changes(rx: &mut mpsc::Receiver<ServerMessage>) -> (Option<uuid::Uuid>, Vec<String>) {
    match rx.recv().await {
        Some(ServerMessage::ChangesList {
            repo_id, unstaged, ..
        }) => (
            repo_id,
            unstaged.into_iter().map(|entry| entry.path).collect(),
        ),
        other => panic!("expected ChangesList, got {:?}", other),
    }
}

async fn recv_history(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> (Option<uuid::Uuid>, Option<String>) {
    match rx.recv().await {
        Some(ServerMessage::CommitHistory {
            repo_id, commits, ..
        }) => (
            repo_id,
            commits
                .first()
                .map(|CommitInfo { message, .. }| message.clone()),
        ),
        other => panic!("expected CommitHistory, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_changes_rejects_stale_local_selector() -> anyhow::Result<()> {
    let (_dir, state, default_id, _test_id) = build_state()?;
    seed_pending(state.repo.as_ref(), "test", "notes/a.md", "hello");
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));

    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_repo.as_deref(), Some("test"));
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn commit_history_rejects_stale_local_selector() -> anyhow::Result<()> {
    let (dir, state, default_id, test_id) = build_state()?;
    write_workspace_file(&dir, "test", "notes/a.md", "hello");
    seed_pending(state.repo.as_ref(), "test", "notes/a.md", "hello");
    let selector = RepoSelector {
        repo_id: Some(test_id),
        repo_name: Some("test".into()),
    };
    state
        .repo
        .stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/a.md"))?;
    state.repo.commit_staged_in_repo(&selector, "initial")?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("test".into(), Some(default_id));

    handle_get_commit_history(&state, &ch, &mut session, "req-1".into(), 10).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoContextInvalid
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert_eq!(session.active_repo_id, Some(default_id));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_commit_history_is_allowed() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: test_id,
            name: "shadow-notes".into(),
            url: Some("urn:test".into()),
        },
    )?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));

    handle_get_commit_history(&state, &ch, &mut session, "req-1".into(), 10).await;

    let (repo_id, first_message) = recv_history(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert_eq!(first_message, None);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_changes_are_allowed_without_locked_db() -> anyhow::Result<()> {
    let (_dir, state, _default_id, test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    state.repo.ensure_shadow_repo_info(
        &peer_id,
        &RepoInfo {
            uuid: test_id,
            name: "shadow-notes".into(),
            url: Some("urn:test".into()),
        },
    )?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));

    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;

    let (repo_id, paths) = recv_changes(&mut uni_rx).await;
    assert_eq!(repo_id, Some(test_id));
    assert!(paths.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_changes_without_repo_selection_clear_stale_db_and_sync_binding()
-> anyhow::Result<()> {
    let (_dir, state, default_id, _test_id) = build_state()?;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some("peer-a".into()));
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(default_id);
    session.set_sync_scope_nonce(13);

    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::ScRepoNotSelected
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
