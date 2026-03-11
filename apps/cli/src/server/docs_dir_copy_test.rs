use super::handlers::docs::handle_copy_doc;
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn copy_dir_recovers_from_missing_source_projection() -> anyhow::Result<()> {
    let (dir, state, repo_id) = build_state()?;
    seed_file(&state, "notes/a.md", "hello")?;
    seed_file(&state, "notes/sub/b.md", "world")?;
    let (uni_tx, _uni_rx) = mpsc::channel(32);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_repo(state.repo.local_repo_name().to_string(), Some(repo_id));
    handle_copy_doc(&state, &ch, &mut session, "notes".into(), "mirror".into()).await;
    assert_eq!(
        std::fs::read_to_string(dir.path().join("vault/default/mirror/a.md"))?,
        "hello"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("vault/default/mirror/sub/b.md"))?,
        "world"
    );
    assert!(state.repo.get_docid("mirror/a.md")?.is_some());
    assert!(state.repo.get_docid("mirror/sub/b.md")?.is_some());
    Ok(())
}
