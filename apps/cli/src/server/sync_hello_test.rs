use super::handlers::sync::{SyncHelloInput, handle_sync_hello};
use super::{AppState, channel::DualChannel, security, tree_state::RepoTreeRegistry};
use deve_core::config::SyncMode;
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::protocol::ServerMessage;
use deve_core::security::IdentityKeyPair;
use deve_core::sync::repo_scoped::RepoScopedSyncEngine;
use deve_core::sync::vector::VersionVector;
use std::sync::Arc;
use tempfile::{TempDir, tempdir};
use tokio::sync::{broadcast, mpsc};

fn build_state() -> anyhow::Result<(TempDir, Arc<AppState>, uuid::Uuid)> {
    let dir = tempdir()?;
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
    let repo = Arc::new(repo);
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
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
                identity_key.peer_id(),
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

fn signed_hello(remote: &IdentityKeyPair, vector: &VersionVector) -> SyncHelloInput {
    let peer_id = remote.peer_id();
    let sorted_map: std::collections::BTreeMap<_, _> = vector.iter().collect();
    let vec_bytes = serde_json::to_vec(&sorted_map).expect("serialize vector");
    let mut msg = Vec::new();
    msg.extend_from_slice(b"deve-handshake");
    msg.extend_from_slice(peer_id.as_str().as_bytes());
    msg.extend_from_slice(&vec_bytes);
    SyncHelloInput {
        peer_id,
        pub_key: remote.public_key_bytes().to_vec(),
        signature: remote.sign(&msg),
        remote_vector: vector.clone(),
        repo_id: uuid::Uuid::nil(),
    }
}

fn seed_local_op(state: &Arc<AppState>) -> anyhow::Result<()> {
    let repo_name = state.repo.local_repo_name().to_string();
    let doc_id =
        state
            .repo
            .apply_file_structure_in_local_repo(&repo_name, "notes/a.md", None, "test")?;
    state.repo.append_generated_op_in_local_repo(
        &repo_name,
        doc_id,
        state.identity_key.peer_id(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "hello".into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )?;
    Ok(())
}

async fn collect_unicast_messages(
    rx: &mut mpsc::Receiver<ServerMessage>,
) -> anyhow::Result<Vec<ServerMessage>> {
    let first = rx.recv().await.expect("at least one message");
    let mut messages = vec![first];
    while let Ok(msg) = rx.try_recv() {
        messages.push(msg);
    }
    Ok(messages)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_creates_named_shadow_repo() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = repo_id;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let _ = uni_rx.recv().await;

    assert_eq!(
        state.repo.list_repos(Some(&remote.peer_id()))?,
        vec!["notes".to_string()]
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_does_not_create_shadow_repo() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = repo_id;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();
    session.mark_browser_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let _ = uni_rx.recv().await;

    assert!(state.repo.list_repos(Some(&remote.peer_id()))?.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_skips_sync_payload_messages() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    seed_local_op(&state)?;
    let remote = IdentityKeyPair::generate();
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = repo_id;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();
    session.mark_browser_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let messages = collect_unicast_messages(&mut uni_rx).await?;

    assert!(
        matches!(messages.first(), Some(ServerMessage::SyncHello { .. })),
        "unexpected first message: {:?}",
        messages.first()
    );
    assert!(!messages.iter().any(|msg| {
        matches!(
            msg,
            ServerMessage::SyncRequest { .. }
                | ServerMessage::SyncSnapshotRequest { .. }
                | ServerMessage::SyncPush { .. }
                | ServerMessage::SyncPushSnapshot { .. }
        )
    }));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn browser_sync_hello_refreshes_shadow_list_without_self_peer() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    state
        .repo
        .ensure_shadow_repo_binding(&remote.peer_id(), repo_id)?;
    let mut hello = signed_hello(&remote, &VersionVector::new());
    hello.repo_id = repo_id;
    let (uni_tx, mut uni_rx) = mpsc::channel(16);
    let ch = DualChannel::new(state.tx.clone(), uni_tx);
    let mut session = super::session::WsSession::new();
    session.mark_browser_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let messages = collect_unicast_messages(&mut uni_rx).await?;

    assert!(
        matches!(messages.first(), Some(ServerMessage::SyncHello { .. })),
        "unexpected first message: {:?}",
        messages.first()
    );
    let shadow_list = messages
        .into_iter()
        .find_map(|msg| match msg {
            ServerMessage::ShadowList { shadows, .. } => Some(shadows),
            _ => None,
        })
        .expect("browser sync hello should refresh shadow list");
    assert!(
        !shadow_list.contains(&remote.peer_id().to_string()),
        "shadow list should not contain self peer: {:?}",
        shadow_list
    );
    Ok(())
}
