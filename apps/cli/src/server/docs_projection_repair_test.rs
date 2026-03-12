use super::handlers::docs::{handle_copy_doc, handle_create_doc, handle_rename_doc};
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let (tx, _rx) = broadcast::channel(32);
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
        repo_id,
    ))
}

fn seed_file(state: &Arc<AppState>, path: &str, content: &str) -> anyhow::Result<()> {
    let doc_id = state.repo.apply_file_structure_in_local_repo(
        state.repo.local_repo_name(),
        path,
        None,
        "test",
    )?;
    state.repo.append_generated_op_in_local_repo(
        state.repo.local_repo_name(),
        doc_id,
        PeerId::new("test-peer"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                PeerId::new("test-peer"),
                seq,
                None,
                None,
            )
        },
    )?;
    Ok(())
}

fn activate_local_repo(session: &mut WsSession, repo: &RepoManager, repo_id: uuid::Uuid) {
    session.switch_repo(repo.local_repo_name().to_string(), Some(repo_id));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rename_recovers_from_missing_source_projection() -> anyhow::Result<()> {
    let (dir, state, repo_id) = build_state()?;
    seed_file(&state, "notes/a.md", "hello")?;
    let (uni_tx, _uni_rx) = mpsc::channel(32);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    activate_local_repo(&mut session, state.repo.as_ref(), repo_id);
    handle_rename_doc(
        &state,
        &ch,
        &mut session,
        "notes/a.md".into(),
        "notes/b.md".into(),
    )
    .await;
    assert_eq!(state.repo.get_docid("notes/a.md")?, None);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("vault/default/notes/b.md"))?,
        "hello"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn copy_recovers_from_missing_source_projection() -> anyhow::Result<()> {
    let (dir, state, repo_id) = build_state()?;
    seed_file(&state, "notes/a.md", "hello")?;
    let (uni_tx, _uni_rx) = mpsc::channel(32);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    activate_local_repo(&mut session, state.repo.as_ref(), repo_id);
    handle_copy_doc(
        &state,
        &ch,
        &mut session,
        "notes/a.md".into(),
        "notes/b.md".into(),
    )
    .await;
    assert_eq!(
        std::fs::read_to_string(dir.path().join("vault/default/notes/b.md"))?,
        "hello"
    );
    assert!(state.repo.get_docid("notes/b.md")?.is_some());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_rejects_existing_tracked_path_without_projection() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    seed_file(&state, "notes/a.md", "ledger only")?;
    let (uni_tx, _uni_rx) = mpsc::channel(32);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    activate_local_repo(&mut session, state.repo.as_ref(), repo_id);
    let original = state.repo.get_docid("notes/a.md")?;
    handle_create_doc(&state, &ch, &mut session, "notes/a.md".into()).await;
    assert_eq!(state.repo.get_docid("notes/a.md")?, original);
    Ok(())
}
