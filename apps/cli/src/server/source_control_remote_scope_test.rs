use super::handlers::source_control::handle_get_doc_diff;
use super::{
    AppState, channel::DualChannel, security, session::WsSession, tree_state::RepoTreeRegistry,
};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoInfo;
use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOC_OPS, LEDGER_OPS};
use deve_core::models::{DocId, LedgerEntry, Op, PeerId};
use deve_core::protocol::{ScPathTarget, ServerMessage};
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    repo.set_vault_root(&vault);
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
        test_id,
    ))
}

fn seed_shadow_doc(
    repo: &RepoManager,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
) -> anyhow::Result<DocId> {
    seed_shadow_doc_with_url(repo, peer_id, repo_id, "shadow-notes", "urn:test")
}

fn seed_shadow_doc_with_url(
    repo: &RepoManager,
    peer_id: &PeerId,
    repo_id: uuid::Uuid,
    repo_name: &str,
    repo_url: &str,
) -> anyhow::Result<DocId> {
    repo.ensure_shadow_repo_info(
        peer_id,
        &RepoInfo {
            uuid: repo_id,
            name: repo_name.into(),
            url: Some(repo_url.into()),
        },
    )?;
    let doc_id = DocId::new();
    repo.run_on_shadow_repo_by_id(peer_id, &repo_id, |db| {
        let _ = deve_core::ledger::node_meta::ensure_file_node(db, "notes/a.md", doc_id)?;
        let entry = LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: "remote".into(),
            },
            1,
            peer_id.clone(),
            1,
            None,
            None,
        );
        let bytes = bincode::serialize(&entry)?;
        let write = db.begin_write()?;
        write
            .open_table(LEDGER_OPS)?
            .insert(1u64, bytes.as_slice())?;
        write
            .open_multimap_table(DOC_OPS)?
            .insert(doc_id.as_u128(), 1u64)?;
        write.commit()?;
        Ok(())
    })?;
    Ok(doc_id)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn readonly_remote_diff_is_allowed_without_locked_db() -> anyhow::Result<()> {
    let (_dir, state, test_id) = build_state()?;
    let peer_id = PeerId::new("peer-a");
    let doc_id = seed_shadow_doc(state.repo.as_ref(), &peer_id, test_id)?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-notes".into(), Some(test_id));

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
            repo_id,
            new_content,
            ..
        }) => {
            assert_eq!(repo_id, Some(test_id));
            assert_eq!(new_content, "remote");
        }
        other => panic!("expected DocDiff, got {:?}", other),
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_diff_without_repo_selection_clears_stale_db_and_sync_binding() -> anyhow::Result<()>
{
    let (_dir, state, test_id) = build_state()?;
    let local_handle = state
        .repo
        .open_database(None, state.repo.local_repo_name())?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some("peer-a".into()));
    session.set_active_db(local_handle);
    session.set_authenticated(PeerId::new("stale-peer"));
    session.bind_repo(test_id);
    session.set_sync_scope_nonce(17);

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "req-miss".into(),
        ScPathTarget::from_path("notes/a.md"),
    )
    .await;

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_diff_fails_closed_when_no_local_counterpart_repo_exists() -> anyhow::Result<()> {
    let (_dir, state, _test_id) = build_state()?;
    let peer_id = PeerId::new("peer-remote-only");
    let remote_repo_id = uuid::Uuid::new_v4();
    let doc_id = seed_shadow_doc_with_url(
        state.repo.as_ref(),
        &peer_id,
        remote_repo_id,
        "shadow-remote-only",
        "urn:remote-only",
    )?;
    let (uni_tx, mut uni_rx) = mpsc::channel(8);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = WsSession::new();
    session.switch_branch(Some(peer_id.to_string()));
    session.switch_repo("shadow-remote-only".into(), Some(remote_repo_id));

    handle_get_doc_diff(
        &state,
        &ch,
        &mut session,
        "req-no-local".into(),
        ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(doc_id),
        },
    )
    .await;

    match uni_rx.recv().await {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(
                error.code,
                deve_core::protocol::ServerErrorCode::StorageNotFound
            );
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| detail.contains("No local repository matched"))
            );
        }
        other => panic!("expected ProtocolError, got {:?}", other),
    }
    Ok(())
}
