//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::super::handlers::sync::handle_sync_hello;
use super::super::sync_hello_test_support::{
    block_shadow_peer_dir, build_state, empty_session, recv_protocol_error, signed_hello_for_repo,
    signed_hello_for_scope, unicast_channel,
};
use deve_core::protocol::{ServerErrorCode, SessionProof};
use deve_core::security::IdentityKeyPair;

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
    assert!(!state
        .repo
        .remotes_dir()
        .join(remote.peer_id().to_filename())
        .try_exists()?);
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
    assert!(!state
        .repo
        .remotes_dir()
        .join(remote.peer_id().to_filename())
        .try_exists()?);
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
    assert!(!state
        .repo
        .remotes_dir()
        .join(remote.peer_id().to_filename())
        .try_exists()?);
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
