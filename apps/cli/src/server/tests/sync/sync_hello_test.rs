//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::handlers::sync::handle_sync_hello;
use super::sync_hello_test_support::{
    block_shadow_peer_dir, build_state, collect_unicast_messages, empty_session,
    recv_protocol_error, signed_hello, signed_hello_for_repo, signed_hello_for_scope,
    unicast_channel,
};
use deve_core::config::GitBridgeMode;
use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::traits::RepoSelector;
use deve_core::models::{DocId, LedgerEntry, Op, PeerId, VersionVector};
use deve_core::protocol::{ScPathTarget, ServerErrorCode, ServerMessage, SessionProof};
use deve_core::security::IdentityKeyPair;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, SourceControlApi};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_creates_repo_scoped_shadow_without_borrowing_local_metadata()
-> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&remote, repo_id);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let _ = rx.recv().await;

    assert!(state.repo.list_repos(Some(&remote.peer_id()))?.is_empty());
    assert!(
        state
            .repo
            .remotes_dir()
            .join(remote.peer_id().to_filename())
            .join(format!("{repo_id}.redb"))
            .exists()
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_binds_session_sync_scope_nonce() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_scope(&remote, repo_id, 9);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let _ = collect_unicast_messages(&mut rx).await?;

    assert_eq!(session.sync_scope_nonce(), Some(9));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_response_refreshes_vector_from_ledger_heads() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let local_peer = state.identity_key.peer_id();
    state.sync_engine.get_or_create_strict(repo_id)?;
    let doc_id = DocId::new();
    state.repo.append_generated_op_in_local_repo(
        state.repo.local_repo_name(),
        doc_id,
        local_peer.clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "local".into(),
                },
                1,
                local_peer.clone(),
                seq,
                None,
                None,
            )
        },
    )?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&remote, repo_id);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let messages = collect_unicast_messages(&mut rx).await?;
    let response_vector = messages
        .iter()
        .find_map(|msg| match msg {
            ServerMessage::SyncHello { vector, .. } => Some(vector),
            _ => None,
        })
        .expect("sync hello response");

    assert_eq!(response_vector.get(&state.identity_key.peer_id()), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_followup_request_carries_known_vector() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let local_peer = state.identity_key.peer_id();
    let doc_id = DocId::new();
    state.repo.append_generated_op_in_local_repo(
        state.repo.local_repo_name(),
        doc_id,
        local_peer.clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "local".into(),
                },
                1,
                local_peer.clone(),
                seq,
                None,
                None,
            )
        },
    )?;

    let remote = IdentityKeyPair::generate();
    let mut remote_vector = VersionVector::new();
    remote_vector.update(remote.peer_id(), 3);
    let mut hello = signed_hello(&remote, &remote_vector);
    hello.repo_id = repo_id;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let messages = collect_unicast_messages(&mut rx).await?;
    let known_vector = messages
        .iter()
        .find_map(|msg| match msg {
            ServerMessage::SyncRequest { known_vector, .. } => Some(known_vector),
            _ => None,
        })
        .expect("sync request follow-up");

    assert_eq!(known_vector.get(&local_peer), 1);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_pushes_source_control_commit_to_full_peer() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let repo_name = state.repo.local_repo_name().to_string();
    let local_peer = state.identity_key.peer_id();
    let path = "p2p-mesh/source-control.md";
    let content = "source-control mesh payload";
    let abs = state.repo.local_repo_workspace_path(&repo_name, path)?;
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&abs, content)?;
    state.repo.run_on_local_repo(&repo_name, |db| {
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
    })?;
    let selector = RepoSelector::default();
    state
        .repo
        .stage_pending_in_repo(&selector, &ScPathTarget::from_path(path))?;
    state.repo.commit_staged_in_repo_with_git_bridge(
        &selector,
        "source control mesh commit",
        GitBridgeMode::Off,
    )?;

    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&remote, repo_id);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let messages = collect_unicast_messages(&mut rx).await?;
    let push = messages
        .iter()
        .find_map(|msg| match msg {
            ServerMessage::SyncPush {
                source_peer_id,
                repo_id,
                encrypted_payload,
                ..
            } => Some((source_peer_id, repo_id, encrypted_payload)),
            _ => None,
        })
        .expect("source-control commit should be offered as SyncPush");

    assert_eq!(push.0, &local_peer);
    assert_eq!(push.1, &repo_id);
    assert!(!push.2.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_fullpeer_offer_set_excludes_third_party_shadow_sources() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let local_peer = state.identity_key.peer_id();
    append_local_op(&state, repo_id)?;
    let third_party = PeerId::new("peer-a");
    append_remote_shadow_op(&state, repo_id, &third_party)?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&remote, repo_id);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let messages = collect_unicast_messages(&mut rx).await?;
    let offered_pushes: Vec<_> = messages
        .iter()
        .filter_map(|msg| match msg {
            ServerMessage::SyncPush { source_peer_id, .. } => Some(source_peer_id.clone()),
            _ => None,
        })
        .collect();

    assert!(offered_pushes.contains(&local_peer));
    assert!(!offered_pushes.contains(&third_party));
    assert!(session.allows_sync_export_source(&local_peer));
    assert!(!session.allows_sync_export_source(&third_party));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_fullpeer_request_set_excludes_third_party_shadow_sources(
) -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    state.sync_engine.get_or_create_strict(repo_id)?;
    let remote = IdentityKeyPair::generate();
    let third_party = PeerId::new("peer-a");
    let mut remote_vector = VersionVector::new();
    remote_vector.update(remote.peer_id(), 1);
    remote_vector.update(third_party.clone(), 1);
    let mut hello = signed_hello(&remote, &remote_vector);
    hello.repo_id = repo_id;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;
    let messages = collect_unicast_messages(&mut rx).await?;
    let requested_sources: Vec<_> = messages
        .iter()
        .flat_map(|msg| match msg {
            ServerMessage::SyncRequest { requests, .. } => requests
                .iter()
                .map(|(peer_id, _)| peer_id.clone())
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect();

    assert!(requested_sources.contains(&remote.peer_id()));
    assert!(!requested_sources.contains(&third_party));
    assert!(session.allows_sync_source(&remote.peer_id()));
    assert!(!session.allows_sync_source(&third_party));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_unknown_repo_before_binding_session() -> anyhow::Result<()> {
    let (_dir, state, _repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let hello = signed_hello_for_repo(&remote, uuid::Uuid::new_v4());
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::ScRepoContextInvalid);
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    assert!(
        !state
            .repo
            .remotes_dir()
            .join(remote.peer_id().to_filename())
            .try_exists()?
    );
    Ok(())
}

fn append_local_op(state: &std::sync::Arc<super::AppState>, repo_id: uuid::Uuid) -> anyhow::Result<()> {
    state.sync_engine.get_or_create_strict(repo_id)?;
    let doc_id = DocId::new();
    let local_peer = state.identity_key.peer_id();
    state.repo.append_generated_op_in_local_repo(
        state.repo.local_repo_name(),
        doc_id,
        local_peer.clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "local".into(),
                },
                1,
                local_peer.clone(),
                seq,
                None,
                None,
            )
        },
    )?;
    Ok(())
}

fn append_remote_shadow_op(
    state: &std::sync::Arc<super::AppState>,
    repo_id: uuid::Uuid,
    remote_peer: &PeerId,
) -> anyhow::Result<()> {
    let doc_id = DocId::new();
    let entry = LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos: 0,
            content: "remote-shadow".into(),
        },
        1,
        remote_peer.clone(),
        1,
        None,
        None,
    );
    state
        .repo
        .append_remote_ops(remote_peer, &repo_id, &[entry])?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_invalid_session_proof_before_binding_session() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let mut hello = signed_hello_for_repo(&remote, repo_id);
    hello.session_proof = SessionProof::new(vec![0; 64]);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    assert!(
        !state
            .repo
            .remotes_dir()
            .join(remote.peer_id().to_filename())
            .try_exists()?
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_rejects_peer_pubkey_mismatch_before_binding_session() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    let other = IdentityKeyPair::generate();
    let mut hello = signed_hello_for_repo(&remote, repo_id);
    hello.peer_pubkey = other.public_key_bytes().to_vec();
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    assert!(
        !state
            .repo
            .remotes_dir()
            .join(remote.peer_id().to_filename())
            .try_exists()?
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_fails_closed_when_shadow_binding_fails() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let remote = IdentityKeyPair::generate();
    block_shadow_peer_dir(&state, &remote)?;
    let hello = signed_hello_for_scope(&remote, repo_id, 7);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = empty_session();

    handle_sync_hello(&state, &ch, &mut session, hello).await;

    let _ = recv_protocol_error(&mut rx).await;
    assert!(session.authenticated_peer_id.is_none());
    assert!(session.bound_repo_id.is_none());
    assert_eq!(session.sync_scope_nonce(), None);
    Ok(())
}
