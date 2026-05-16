use super::enqueue_with_limit;
use super::is_write_message;
use super::prepare_queue_for_new_connection;
use deve_core::models::{PeerId, VersionVector};
use deve_core::protocol::ClientMessage;
use deve_core::protocol::ScPathTarget;
use std::collections::VecDeque;

#[test]
fn output_queue_drops_oldest_message_at_capacity() {
    let mut queue = VecDeque::new();
    for _ in 0..500 {
        enqueue_with_limit(&mut queue, ClientMessage::Ping);
    }

    enqueue_with_limit(
        &mut queue,
        ClientMessage::ListDocs {
            request_id: "latest".into(),
            scope_nonce: Some(1),
        },
    );

    assert_eq!(queue.len(), 500);
    assert!(matches!(queue.front(), Some(ClientMessage::Ping)));
    assert!(matches!(
        queue.back(),
        Some(ClientMessage::ListDocs { request_id, .. }) if request_id == "latest"
    ));
}

#[test]
fn prepare_queue_for_new_connection_keeps_reads_and_prepends_ping() {
    let mut queue = VecDeque::from([
        ClientMessage::StageFile {
            target: ScPathTarget::from_path("note.md"),
            scope_nonce: Some(1),
        },
        ClientMessage::ListDocs {
            request_id: "docs".into(),
            scope_nonce: Some(1),
        },
        ClientMessage::GetChanges {
            request_id: "changes".into(),
            scope_nonce: Some(1),
        },
    ]);

    prepare_queue_for_new_connection(&mut queue);

    assert!(matches!(queue.front(), Some(ClientMessage::Ping)));
    assert_eq!(queue.len(), 3);
    assert!(
        queue
            .iter()
            .all(|msg| !matches!(msg, ClientMessage::StageFile { .. }))
    );
}

#[test]
fn output_write_classification_distinguishes_reads_from_writes() {
    assert!(is_write_message(&ClientMessage::CreateDoc {
        name: "Untitled.md".into(),
        scope_nonce: Some(1),
    }));
    assert!(is_write_message(&ClientMessage::StageFile {
        target: ScPathTarget::from_path("note.md"),
        scope_nonce: Some(1),
    }));
    assert!(is_write_message(&ClientMessage::RegisterWriter {
        peer_id: PeerId::new("browser-peer"),
        repo_id: uuid::Uuid::nil(),
        scope_nonce: 1.into(),
    }));
    assert!(is_write_message(&ClientMessage::SyncPushSnapshot {
        source_peer_id: PeerId::new("browser-peer"),
        repo_id: uuid::Uuid::nil(),
        server_vector: VersionVector::new(),
        snapshot_kind: None,
        source_proof: None,
        payload: vec![],
    }));
    assert!(!is_write_message(&ClientMessage::Ping));
    assert!(!is_write_message(&ClientMessage::SyncRequest {
        repo_id: uuid::Uuid::nil(),
        known_vector: VersionVector::new(),
        requests: vec![],
    }));
    assert!(!is_write_message(&ClientMessage::ListRepos {
        request_id: "repos".into(),
        scope_nonce: Some(1),
    }));
}
