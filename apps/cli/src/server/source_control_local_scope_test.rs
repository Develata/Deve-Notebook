use super::handlers::source_control::handle_get_doc_diff;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::PeerId;
use deve_core::protocol::{ScPathTarget, ServerMessage};
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
async fn local_diff_resolves_renamed_target_before_reading_workspace() -> anyhow::Result<()> {
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
    state.repo.commit_staged("initial")?;
    let doc_id = state
        .repo
        .get_docid("notes/a.md")?
        .expect("existing doc id");

    std::fs::remove_file(dir.path().join("vault").join("default").join("notes/a.md"))?;
    write_workspace_file(&dir, "notes/b.md", "hello renamed");
    state
        .repo
        .run_on_local_repo(state.repo.local_repo_name(), |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "notes/a.md".into(),
                    renamed_from: None,
                    doc_id: Some(doc_id),
                    change_type: ChangeStatus::Deleted,
                    content_hash: String::new(),
                    detected_at: 2,
                    has_conflict: false,
                },
            )?;
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "notes/b.md".into(),
                    renamed_from: Some("notes/a.md".into()),
                    doc_id: Some(doc_id),
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("hello renamed"),
                    detected_at: 2,
                    has_conflict: false,
                },
            )
        })?;

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("default".into(), None);

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "req-1".into(),
        ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(doc_id),
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::DocDiff {
            path,
            old_content,
            new_content,
            ..
        }) => {
            assert_eq!(path, "notes/b.md");
            assert_eq!(old_content, "hello");
            assert_eq!(new_content, "hello renamed");
        }
        other => panic!("expected DocDiff, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn local_diff_rejects_reused_path_when_doc_id_misses() -> anyhow::Result<()> {
    let (dir, state) = build_state()?;
    write_workspace_file(&dir, "notes/a.md", "hello");
    state
        .repo
        .apply_file_structure_in_local_repo("default", "notes/a.md", None, "test")?;
    let tracked_doc_id = state.repo.get_docid("notes/a.md")?.expect("tracked doc id");
    state.repo.append_generated_op_in_local_repo(
        "default",
        tracked_doc_id,
        PeerId::new("test"),
        |seq| {
            deve_core::models::LedgerEntry::new_content(
                tracked_doc_id,
                deve_core::models::Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                1,
                PeerId::new("test"),
                seq,
                None,
                None,
            )
        },
    )?;
    state.repo.apply_file_delete_structure_in_local_repo(
        "default",
        "notes/a.md",
        Some(tracked_doc_id),
        "test",
    )?;
    let reused_doc_id = state.repo.apply_file_structure_in_local_repo(
        "default",
        "notes/reused.md",
        None,
        "test",
    )?;
    state.repo.append_generated_op_in_local_repo(
        "default",
        reused_doc_id,
        PeerId::new("test"),
        |seq| {
            deve_core::models::LedgerEntry::new_content(
                reused_doc_id,
                deve_core::models::Op::Insert {
                    pos: 0,
                    content: "other".into(),
                },
                1,
                PeerId::new("test"),
                seq,
                None,
                None,
            )
        },
    )?;
    write_workspace_file(&dir, "notes/reused.md", "other");

    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo("default".into(), None);

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "req-2".into(),
        ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(tracked_doc_id),
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert!(
                error
                    .detail
                    .as_deref()
                    .unwrap_or_default()
                    .contains("Source control target not resolved")
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}
