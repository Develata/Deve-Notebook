//! plan_ref:
//!   - 05_network#server-ws-runtime

use super::sync_hello_test_support::signed_hello_for_scope;
use super::ws_protocol_acceptance_support::{
    WsHarness, connect_harness, recv_server_message, send_client_message,
};
use deve_core::models::PeerId;
use deve_core::protocol::{ClientMessage, ServerMessage};
use deve_core::security::IdentityKeyPair;

const SWITCH_NONCE: u64 = 1;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ws_endpoint_sync_hello_uses_switched_repo_scope() -> anyhow::Result<()> {
    let harness = WsHarness::spawn().await?;
    let repo_id = harness.repo_id;
    let mut ws = connect_harness(&harness).await?;

    send_client_message(
        &mut ws,
        ClientMessage::SwitchRepoExact {
            name: "notes".into(),
            repo_id,
            switch_nonce: Some(SWITCH_NONCE),
        },
    )
    .await?;
    assert_repo_switched(recv_server_message(&mut ws).await?, repo_id);
    let doc_list = recv_server_message(&mut ws).await?;
    let tree_update = recv_server_message(&mut ws).await?;
    assert_repo_view_messages(doc_list, tree_update, repo_id);

    let remote = IdentityKeyPair::generate();
    send_client_message(&mut ws, client_sync_hello(&remote, repo_id)).await?;
    let sync_hello = recv_server_message(&mut ws).await?;
    assert_sync_hello(sync_hello, repo_id, &harness.local_peer_id);
    assert_shadow_list(recv_server_message(&mut ws).await?, &remote);

    harness.shutdown().await;
    Ok(())
}

fn client_sync_hello(remote: &IdentityKeyPair, repo_id: uuid::Uuid) -> ClientMessage {
    let hello = signed_hello_for_scope(remote, repo_id, SWITCH_NONCE);
    ClientMessage::SyncHello {
        peer_id: hello.peer_id,
        pub_key: hello.pub_key,
        signature: hello.signature,
        vector: hello.remote_vector,
        repo_id: hello.repo_id,
        scope_nonce: hello.scope_nonce,
    }
}

fn assert_repo_switched(message: ServerMessage, repo_id: uuid::Uuid) {
    match message {
        ServerMessage::RepoSwitched {
            branch,
            name,
            uuid,
            switch_nonce,
        } => {
            assert_eq!(branch, None);
            assert_eq!(name, "notes");
            assert_eq!(uuid, repo_id.to_string());
            assert_eq!(switch_nonce, Some(SWITCH_NONCE));
        }
        other => panic!("expected RepoSwitched, got {other:?}"),
    }
}

fn assert_repo_view_messages(first: ServerMessage, second: ServerMessage, repo_id: uuid::Uuid) {
    let (doc_repo, doc_branch, doc_nonce, tree_repo, tree_branch, tree_nonce) =
        match (first, second) {
            (
                ServerMessage::DocList {
                    repo_id: Some(doc_repo),
                    branch: doc_branch,
                    scope_nonce: doc_nonce,
                    ..
                },
                ServerMessage::TreeUpdate {
                    repo_id: Some(tree_repo),
                    branch: tree_branch,
                    scope_nonce: tree_nonce,
                    ..
                },
            ) => (doc_repo, doc_branch, doc_nonce, tree_repo, tree_branch, tree_nonce),
            other => panic!("expected DocList then TreeUpdate, got {other:?}"),
        };
    assert_eq!((doc_repo, tree_repo), (repo_id, repo_id));
    assert_eq!((doc_branch, tree_branch), (None, None));
    assert_eq!((doc_nonce, tree_nonce), (Some(SWITCH_NONCE), Some(SWITCH_NONCE)));
}

fn assert_sync_hello(message: ServerMessage, repo_id: uuid::Uuid, peer_id: &PeerId) {
    match message {
        ServerMessage::SyncHello {
            peer_id: actual_peer,
            repo_id: actual,
            scope_nonce,
            pub_key,
            signature,
            ..
        } => {
            assert_eq!(&actual_peer, peer_id);
            assert_eq!(actual, repo_id);
            assert_eq!(scope_nonce, SWITCH_NONCE);
            assert!(!pub_key.is_empty());
            assert!(!signature.is_empty());
        }
        other => panic!("expected SyncHello, got {other:?}"),
    }
}

fn assert_shadow_list(message: ServerMessage, remote: &IdentityKeyPair) {
    match message {
        ServerMessage::ShadowList {
            scope_nonce,
            shadows,
            ..
        } => {
            assert_eq!(scope_nonce, Some(SWITCH_NONCE));
            assert!(!shadows.contains(&remote.peer_id().to_string()));
        }
        other => panic!("expected ShadowList, got {other:?}"),
    }
}
