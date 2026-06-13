use super::super::transfer::{send_requested_ops, send_requested_snapshot};
use super::support::{MockSocket, append_local_op, append_remote_shadow_op, test_state_with_dir};
use crate::server::p2p::ExchangeStats;
use deve_core::models::PeerId;
use deve_core::protocol::frame::decode_client_binary;
use deve_core::protocol::{ClientMessage, SyncPayloadKind};
use deve_core::security::IdentityKeyPair;
use std::sync::Arc;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn p2p_send_requested_ops_signs_local_diff_source() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let local_peer = identity.peer_id();
    let (_dir, state) = test_state_with_dir(identity)?;
    let repo_id = state
        .repo
        .get_repo_info_for(None, Some(state.repo.local_repo_name()))?
        .expect("repo info")
        .uuid;
    append_local_op(&state, repo_id)?;
    let mut socket = MockSocket::new(Vec::new());
    let mut stats = ExchangeStats::default();

    send_requested_ops(
        &state,
        &mut socket,
        repo_id,
        vec![(local_peer.clone(), (1, 2))],
        &mut stats,
    )
    .await?;

    assert_eq!(stats.sent_pushes, 1);
    assert_eq!(socket.sent.len(), 1);
    let Message::Binary(bytes) = &socket.sent[0] else {
        panic!("expected binary P2P frame");
    };
    match decode_client_binary(bytes)? {
        ClientMessage::SyncPush {
            source_peer_id,
            repo_id: actual_repo_id,
            header,
            encrypted_payload,
        } => {
            assert_eq!(source_peer_id, local_peer);
            assert_eq!(actual_repo_id, repo_id);
            assert_eq!(header.peer_id, local_peer);
            assert_eq!(header.payload_kind, SyncPayloadKind::Diff);
            assert!(header.source_proof.is_some());
            header.validate_source_proof(&encrypted_payload, true)?;
        }
        other => panic!("expected SyncPush, got {other:?}"),
    }
    Ok(())
}

#[tokio::test]
async fn p2p_send_requested_snapshot_rejects_unsigned_non_local_source() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let (_dir, state) = test_state_with_dir(identity)?;
    let repo_id = state
        .repo
        .get_repo_info_for(None, Some(state.repo.local_repo_name()))?
        .expect("repo info")
        .uuid;
    let third_party = PeerId::new("peer-a");
    state.sync_engine.get_or_create_strict(repo_id)?;
    append_remote_shadow_op(&state, repo_id, &third_party)?;
    let mut socket = MockSocket::new(Vec::new());
    let mut stats = ExchangeStats::default();

    let err = send_requested_snapshot(
        &state,
        &mut socket,
        third_party,
        repo_id,
        Some("source-boundary-check".into()),
        &mut stats,
    )
    .await
    .expect_err("P2P v1 must not send unsigned third-party snapshot sources");

    assert!(
        err.to_string()
            .contains("cannot sign non-local snapshot source")
    );
    assert!(socket.sent.is_empty());
    assert_eq!(stats.sent_snapshots, 0);
    Ok(())
}
