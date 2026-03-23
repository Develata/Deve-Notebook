use super::handlers::source_control::handle_commit;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::database::DatabaseHandle;
use deve_core::models::PeerId;
use deve_core::protocol::ServerMessage;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let identity_key = security::load_or_generate_identity_key(&dir.path().join("host"))?;
    Ok((
        dir,
        Arc::new(AppState {
            repo: repo.clone(),
            sync_manager: Arc::new(deve_core::sync::SyncManager::new(repo.clone(), vault)),
            tx: broadcast::channel(16).0,
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

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_commit_ack_carries_scope_nonce() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    write_workspace_file(&dir, "notes/a.md", "hello");
    state
        .repo
        .run_on_local_repo(state.repo.local_repo_name(), |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "notes/a.md".into(),
                    renamed_from: None,
                    doc_id: None,
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("hello"),
                    detected_at: 1,
                    has_conflict: false,
                },
            )
        })?;
    state.repo.stage_pending("notes/a.md")?;

    let (uni_tx, _uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut rx = state.tx.subscribe();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.switch_repo("default".into(), None);
    session.set_scope_nonce(Some(23));

    handle_commit(&state, &ch, &mut session, "initial".into()).await;

    match rx.recv().await.expect("broadcast ack") {
        ServerMessage::CommitAck {
            repo_id,
            scope_nonce,
            ..
        } => {
            assert_eq!(repo_id, state.repo.get_repo_info()?.map(|info| info.uuid));
            assert_eq!(scope_nonce, Some(23));
        }
        other => panic!("expected CommitAck, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_commit_bootstraps_after_clearing_stale_runtime_binding() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    write_workspace_file(&dir, "notes/stale.md", "hello");
    state
        .repo
        .run_on_local_repo(state.repo.local_repo_name(), |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "notes/stale.md".into(),
                    renamed_from: None,
                    doc_id: None,
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("hello"),
                    detected_at: 1,
                    has_conflict: false,
                },
            )
        })?;
    state.repo.stage_pending("notes/stale.md")?;

    let stale_db = Arc::new(redb::Database::create(dir.path().join("stale-local.redb"))?);
    let (uni_tx, _uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut rx = state.tx.subscribe();
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(29));
    session.set_active_db(DatabaseHandle {
        db: stale_db,
        readonly: false,
        branch: None,
        repo_id: Some(uuid::Uuid::new_v4()),
        repo_name: "ghost".into(),
    });
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(uuid::Uuid::new_v4());
    session.set_sync_scope_nonce(29);

    handle_commit(&state, &ch, &mut session, "stale".into()).await;

    match rx.recv().await.expect("broadcast ack") {
        ServerMessage::CommitAck {
            repo_id,
            scope_nonce,
            ..
        } => {
            assert_eq!(repo_id, state.repo.get_repo_info()?.map(|info| info.uuid));
            assert_eq!(scope_nonce, Some(29));
        }
        other => panic!("expected CommitAck, got {:?}", other),
    }
    let default_id = state.repo.get_repo_info()?.expect("default info").uuid;
    assert_eq!(session.active_repo.as_deref(), Some("default"));
    assert_eq!(session.active_repo_id, Some(default_id));
    assert!(session.get_active_db().is_none());
    assert!(session.bound_repo_id.is_none());
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.sync_scope_nonce().is_none());
    Ok(())
}
