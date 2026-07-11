//! plan_ref:
//!   - 07_network#server-ws-runtime
//!   - 04_repository#repo-scope-runtime

use super::super::handlers::sync::handle_sync_hello;
use super::super::sync_hello_test_support::{
    build_state, collect_unicast_messages, empty_session, signed_hello, signed_hello_for_repo,
    unicast_channel,
};
use deve_core::models::{DocId, FactActor, Op, VersionVector};
use deve_core::protocol::ServerMessage;
use deve_core::security::IdentityKeyPair;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sync_hello_response_refreshes_vector_from_ledger_heads() -> anyhow::Result<()> {
    let (_dir, state, repo_id) = build_state()?;
    state.sync_engine.get_or_create_strict(repo_id)?;
    let doc_id = DocId::new();
    state
        .repo
        .local_fact_writer(FactActor::new("test")?)
        .append_content_in_local_repo(
            state.repo.local_repo_name(),
            doc_id,
            Op::Insert {
                pos: 0,
                content: "local".into(),
            },
            1,
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
    state
        .repo
        .local_fact_writer(FactActor::new("test")?)
        .append_content_in_local_repo(
            state.repo.local_repo_name(),
            doc_id,
            Op::Insert {
                pos: 0,
                content: "local".into(),
            },
            1,
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
