use super::super::exchange::handle_server_message;
use super::super::hello::signed_sync_hello;
use super::support::{MockSocket, peer, peer_with_id, test_state_with_dir};
use crate::server::p2p::ExchangeStats;
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::{ClientMessage, ScopeNonce, ServerMessage};
use deve_core::security::IdentityKeyPair;
use deve_core::sync::handshake_proof::{sync_hello_transcript, verify_sync_hello_proof};
use std::sync::Arc;

#[test]
fn p2p_mesh_sync_hello_is_signed_for_full_peer_admission_path() {
    let identity = IdentityKeyPair::generate();
    let repo_id = uuid::Uuid::new_v4();
    let hello = signed_sync_hello(&identity, repo_id, VersionVector::new()).expect("signed hello");

    match hello {
        ClientMessage::SyncHello {
            peer_id,
            peer_pubkey,
            session_proof,
            vector,
            repo_id: decoded_repo,
            scope_nonce,
        } => {
            assert_eq!(decoded_repo, repo_id);
            assert_eq!(scope_nonce.get(), 0);
            verify_sync_hello_proof(&peer_id, &peer_pubkey, session_proof.signature(), &vector)
                .expect("client SyncHello proof verifies");
        }
        other => panic!("expected SyncHello, got {other:?}"),
    }
}

#[tokio::test]
async fn p2p_exchange_rejects_configured_peer_id_mismatch() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let (_dir, state) = test_state_with_dir(identity)?;
    let repo_id = uuid::Uuid::new_v4();
    let actual_peer = PeerId::new("actual-peer");
    let message = ServerMessage::SyncHello {
        peer_id: actual_peer,
        repo_id,
        scope_nonce: ScopeNonce::new(0),
        pub_key: Vec::new(),
        signature: Vec::new(),
        vector: VersionVector::new(),
    };
    let mut stats = ExchangeStats::default();
    let mut socket = MockSocket::new(Vec::new());

    let err = handle_server_message(
        &peer_with_id(repo_id, "expected-peer"),
        repo_id,
        &state,
        &mut socket,
        message,
        &mut stats,
    )
    .await
    .expect_err("configured peer_id mismatch must fail closed");

    assert!(err.to_string().contains("configured peer_id"));
    assert!(!stats.saw_hello);
    Ok(())
}

#[tokio::test]
async fn p2p_exchange_rejects_invalid_sync_hello_signature() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let (_dir, state) = test_state_with_dir(identity)?;
    let repo_id = uuid::Uuid::new_v4();
    let remote = IdentityKeyPair::generate();
    let remote_peer = remote.peer_id();
    let message = ServerMessage::SyncHello {
        peer_id: remote_peer.clone(),
        repo_id,
        scope_nonce: ScopeNonce::new(0),
        pub_key: remote.public_key_bytes().to_vec(),
        signature: vec![0; 64],
        vector: VersionVector::new(),
    };
    let mut stats = ExchangeStats::default();
    let mut socket = MockSocket::new(Vec::new());

    let err = handle_server_message(
        &peer_with_id(repo_id, remote_peer.as_str()),
        repo_id,
        &state,
        &mut socket,
        message,
        &mut stats,
    )
    .await
    .expect_err("invalid SyncHello signature must fail closed");

    assert!(err.to_string().contains("Handshake Signature"));
    assert!(!stats.saw_hello);
    Ok(())
}

#[tokio::test]
async fn p2p_exchange_rejects_sync_hello_pubkey_peer_id_mismatch() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let (_dir, state) = test_state_with_dir(identity)?;
    let repo_id = uuid::Uuid::new_v4();
    let claimed = IdentityKeyPair::generate();
    let signer = IdentityKeyPair::generate();
    let claimed_peer = claimed.peer_id();
    let vector = VersionVector::new();
    let transcript =
        sync_hello_transcript(&claimed_peer, &vector).expect("version vector serializes");
    let message = ServerMessage::SyncHello {
        peer_id: claimed_peer.clone(),
        repo_id,
        scope_nonce: ScopeNonce::new(0),
        pub_key: signer.public_key_bytes().to_vec(),
        signature: signer.sign(&transcript),
        vector,
    };
    let mut stats = ExchangeStats::default();
    let mut socket = MockSocket::new(Vec::new());

    let err = handle_server_message(
        &peer_with_id(repo_id, claimed_peer.as_str()),
        repo_id,
        &state,
        &mut socket,
        message,
        &mut stats,
    )
    .await
    .expect_err("SyncHello pubkey/peer_id mismatch must fail closed");

    assert!(err.to_string().contains("PeerID mismatch"));
    assert!(!stats.saw_hello);
    Ok(())
}

#[tokio::test]
async fn p2p_exchange_rejects_authenticated_self_loop() -> anyhow::Result<()> {
    let identity = Arc::new(IdentityKeyPair::generate());
    let self_peer_id = identity.peer_id();
    let (_dir, state) = test_state_with_dir(identity)?;
    let repo_id = uuid::Uuid::new_v4();
    let message = ServerMessage::SyncHello {
        peer_id: self_peer_id,
        repo_id,
        scope_nonce: ScopeNonce::new(0),
        pub_key: Vec::new(),
        signature: Vec::new(),
        vector: VersionVector::new(),
    };
    let mut stats = ExchangeStats::default();
    let mut socket = MockSocket::new(Vec::new());

    let err = handle_server_message(
        &peer(repo_id),
        repo_id,
        &state,
        &mut socket,
        message,
        &mut stats,
    )
    .await
    .expect_err("authenticated self-loop must fail");

    assert!(err.to_string().contains("self-loop"));
    Ok(())
}
