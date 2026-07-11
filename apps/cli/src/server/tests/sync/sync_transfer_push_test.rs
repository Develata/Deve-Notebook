//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::handlers::sync::handle_sync_push as handle_sync_push_inner;
use super::sync_transfer_scope_test_support::{
    append_local_doc, bound_session, build_state, build_state_with_mode,
    encrypted_insert_for_author, recv_protocol_error, unicast_channel,
};
use deve_core::config::SyncMode;
use deve_core::ledger::range;
use deve_core::models::{PeerId, RepoId, VersionVector};
use deve_core::protocol::{ServerErrorCode, SyncPayloadKind, SyncPushHeader};
use deve_core::security::{EncryptedOp, IdentityKeyPair};

async fn handle_sync_push(
    state: &std::sync::Arc<super::AppState>,
    ch: &crate::server::channel::DualChannel,
    session: &mut crate::server::session::WsSession,
    peer_id: PeerId,
    repo_id: RepoId,
    header: SyncPushHeader,
    ops: Vec<EncryptedOp>,
) {
    let start = ops
        .first()
        .map(|op| op.peer_seq)
        .unwrap_or(deve_core::models::PeerFactSeq::ONE);
    let end = ops.last().map(|op| op.peer_seq).unwrap_or(start);
    handle_sync_push_inner(
        state,
        ch,
        session,
        peer_id,
        repo_id,
        (start, end),
        header,
        ops,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn manual_sync_push_buffers_without_applying_remote_ops() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state_with_mode(SyncMode::Manual)?;
    let source_key = IdentityKeyPair::generate();
    let peer = source_key.peer_id();
    let op = encrypted_insert_for_author(&state, repo_id, &peer, 1)?;
    let (ch, _rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(peer.clone()), Some(31));
    session.set_requested_sync_sources([peer.clone()]);

    handle_sync_push(
        &state,
        &ch,
        &mut session,
        peer.clone(),
        repo_id,
        signed_sync_push_header(repo_id, &source_key, std::slice::from_ref(&op))?,
        vec![op],
    )
    .await;

    let pending = state
        .sync_engine
        .with_strict_engine(repo_id, |engine| engine.pending_ops_count())?;
    assert_eq!(pending, 1);
    assert_eq!(state.repo.get_shadow_max_seq(&peer, &repo_id)?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_uses_message_source_peer_for_shadow_write() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let relay_peer = PeerId::new("relay-peer");
    let source_key = IdentityKeyPair::generate();
    let source_peer = source_key.peer_id();
    let op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    let (ch, _rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(relay_peer.clone()), Some(41));
    session.set_requested_sync_sources([source_peer.clone()]);

    handle_sync_push(
        &state,
        &ch,
        &mut session,
        source_peer.clone(),
        repo_id,
        signed_sync_push_header(repo_id, &source_key, std::slice::from_ref(&op))?,
        vec![op],
    )
    .await;

    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 1);
    assert_eq!(state.repo.get_shadow_max_seq(&relay_peer, &repo_id)?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_does_not_pollute_transport_or_local_ledger() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    append_local_doc(&state)?;
    let local_before = state
        .repo
        .run_on_local_repo(state.repo.local_repo_name(), range::get_max_seq)?;
    let relay_peer = PeerId::new("relay-peer");
    let malicious_key = IdentityKeyPair::generate();
    let malicious_source = malicious_key.peer_id();
    let op = encrypted_insert_for_author(&state, repo_id, &malicious_source, 1)?;
    let (ch, _rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(relay_peer.clone()), Some(41));
    session.set_requested_sync_sources([malicious_source.clone()]);

    handle_sync_push(
        &state,
        &ch,
        &mut session,
        malicious_source.clone(),
        repo_id,
        signed_sync_push_header(repo_id, &malicious_key, std::slice::from_ref(&op))?,
        vec![op],
    )
    .await;

    assert_eq!(state.repo.get_shadow_max_seq(&malicious_source, &repo_id)?, 1);
    assert_eq!(state.repo.get_shadow_max_seq(&relay_peer, &repo_id)?, 0);
    assert_eq!(
        state
            .repo
            .run_on_local_repo(state.repo.local_repo_name(), range::get_max_seq)?,
        local_before
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_rejects_relay_forged_source_proof() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let relay_key = IdentityKeyPair::generate();
    let relay_peer = relay_key.peer_id();
    let source_key = IdentityKeyPair::generate();
    let source_peer = source_key.peer_id();
    let op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    let mut header = sync_push_header(repo_id, &source_peer);
    header.source_proof = Some(deve_core::protocol::SyncSourceProof::sign(
        repo_id,
        &relay_peer,
        &VersionVector::new(),
        SyncPayloadKind::Diff,
        std::slice::from_ref(&op),
        &relay_key,
    )?);
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(relay_peer), Some(41));
    session.set_requested_sync_sources([source_peer.clone()]);

    handle_sync_push(
        &state,
        &ch,
        &mut session,
        source_peer.clone(),
        repo_id,
        header,
        vec![op],
    )
    .await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncInvalidPayload);
    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_rejects_indirect_source_without_proof() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let relay_peer = PeerId::new("relay-peer");
    let source_key = IdentityKeyPair::generate();
    let source_peer = source_key.peer_id();
    let op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(relay_peer), Some(41));
    session.set_requested_sync_sources([source_peer.clone()]);

    handle_sync_push(
        &state,
        &ch,
        &mut session,
        source_peer.clone(),
        repo_id,
        sync_push_header(repo_id, &source_peer),
        vec![op],
    )
    .await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncInvalidPayload);
    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_rejects_envelope_seq_mismatch() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let source_peer = PeerId::new("origin-peer");
    let mut op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    op.peer_seq = 2_u64.into();
    let (ch, _rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(source_peer.clone()), Some(41));
    session.set_requested_sync_sources([source_peer.clone()]);

    handle_sync_push(
        &state,
        &ch,
        &mut session,
        source_peer.clone(),
        repo_id,
        sync_push_header(repo_id, &source_peer),
        vec![op],
    )
    .await;

    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_rejects_unrequested_authenticated_source() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let source_peer = PeerId::new("origin-peer");
    let op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(source_peer.clone()), Some(42));

    handle_sync_push(
        &state,
        &ch,
        &mut session,
        source_peer.clone(),
        repo_id,
        sync_push_header(repo_id, &source_peer),
        vec![op],
    )
    .await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_rejects_unrequested_relay_source() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let relay_peer = PeerId::new("relay-peer");
    let source_peer = PeerId::new("origin-peer");
    let op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(relay_peer), Some(42));

    handle_sync_push(
        &state,
        &ch,
        &mut session,
        source_peer.clone(),
        repo_id,
        sync_push_header(repo_id, &source_peer),
        vec![op],
    )
    .await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncPeerUnauthenticated);
    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_rejects_route_header_source_mismatch() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let source_peer = PeerId::new("origin-peer");
    let header_peer = PeerId::new("other-origin");
    let op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(source_peer.clone()), Some(42));
    session.set_requested_sync_sources([source_peer.clone()]);

    handle_sync_push(
        &state,
        &ch,
        &mut session,
        source_peer.clone(),
        repo_id,
        sync_push_header(repo_id, &header_peer),
        vec![op],
    )
    .await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncInvalidPayload);
    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_push_rejects_route_header_payload_kind_mismatch() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    let source_peer = PeerId::new("origin-peer");
    let op = encrypted_insert_for_author(&state, repo_id, &source_peer, 1)?;
    let mut header = sync_push_header(repo_id, &source_peer);
    header.payload_kind = SyncPayloadKind::Snapshot;
    let (ch, mut rx) = unicast_channel(&state);
    let mut session = bound_session(repo_id, Some(source_peer.clone()), Some(42));
    session.set_requested_sync_sources([source_peer.clone()]);

    handle_sync_push(
        &state,
        &ch,
        &mut session,
        source_peer.clone(),
        repo_id,
        header,
        vec![op],
    )
    .await;

    let error = recv_protocol_error(&mut rx).await;
    assert_eq!(error.code, ServerErrorCode::SyncInvalidPayload);
    assert_eq!(state.repo.get_shadow_max_seq(&source_peer, &repo_id)?, 0);
    Ok(())
}

fn sync_push_header(repo_id: RepoId, peer_id: &PeerId) -> SyncPushHeader {
    SyncPushHeader::diff(repo_id, peer_id.clone(), VersionVector::new())
}

fn signed_sync_push_header(
    repo_id: RepoId,
    source_key: &IdentityKeyPair,
    payload: &[EncryptedOp],
) -> Result<SyncPushHeader, deve_core::protocol::SyncSourceProofError> {
    SyncPushHeader::signed_diff(repo_id, source_key.peer_id(), VersionVector::new(), payload, source_key)
}
