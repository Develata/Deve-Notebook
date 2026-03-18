use super::handlers::source_control::handle_get_changes;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, PeerId};
use deve_core::protocol::ServerMessage;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, staging};
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn get_changes_keeps_same_path_entries_for_distinct_doc_ids() -> anyhow::Result<()> {
    let (_dir, state) = build_state()?;
    let deleted_doc = DocId::new();
    let added_doc = DocId::new();
    state.repo.run_on_local_repo("default", |db| {
        staging::stage_pending_entry(
            db,
            &PendingFsEntry {
                path: "notes/reused.md".into(),
                renamed_from: None,
                doc_id: Some(deleted_doc),
                change_type: ChangeStatus::Deleted,
                content_hash: String::new(),
                detected_at: 1,
                has_conflict: false,
            },
        )?;
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/reused.md".into(),
                renamed_from: None,
                doc_id: Some(added_doc),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("new"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.mark_browser_session();
    session.set_scope_nonce(Some(23));
    session.switch_repo("default".into(), None);
    handle_get_changes(&state, &ch, &mut session, Some("req-1".into())).await;

    match uni_rx.recv().await {
        Some(ServerMessage::ChangesList {
            scope_nonce,
            staged,
            unstaged,
            ..
        }) => {
            assert_eq!(scope_nonce, Some(23));
            assert!(staged.iter().any(|entry| {
                entry.path == "notes/reused.md"
                    && entry.doc_id == Some(deleted_doc)
                    && entry.status == ChangeStatus::Deleted
            }));
            assert!(unstaged.iter().any(|entry| {
                entry.path == "notes/reused.md"
                    && entry.doc_id == Some(added_doc)
                    && entry.status == ChangeStatus::Added
            }));
        }
        other => panic!("expected ChangesList, got {:?}", other),
    }
    Ok(())
}
