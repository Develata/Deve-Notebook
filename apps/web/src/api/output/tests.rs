use super::is_write_message;
use super::prepare_queue_for_new_connection;
use super::requeue_front_after_send_failure;
use super::try_enqueue;
use crate::api::outbound_admission::OutboundFrame;
use deve_core::models::{DocId, NodeId, PeerId, RepoId, VersionVector};
use deve_core::protocol::ScPathTarget;
use deve_core::protocol::frame::decode_client_binary;
use deve_core::protocol::{
    ClientMessage, DocumentCreateRequest, RepoControlRequest, RepoLifecycleIntent, ScopeNonce,
    SwitchNonce,
};
use std::collections::VecDeque;

#[test]
fn output_queue_rejects_new_message_at_capacity_without_eviction() {
    let mut queue = VecDeque::new();
    for _ in 0..super::MAX_QUEUE_SIZE {
        try_enqueue(&mut queue, OutboundFrame::for_test(ClientMessage::Ping)).unwrap();
    }

    let rejected = try_enqueue(
        &mut queue,
        OutboundFrame::for_test(ClientMessage::ListDocs {
            request_id: "latest".into(),
            scope_nonce: Some(1),
        }),
    )
    .unwrap_err();

    assert_eq!(queue.len(), super::MAX_QUEUE_SIZE);
    assert!(matches!(
        queue.front().map(OutboundFrame::message_class),
        Some(super::OutboundMessageClass::Keepalive)
    ));
    assert!(matches!(
        queue.back().map(OutboundFrame::message_class),
        Some(super::OutboundMessageClass::Keepalive)
    ));
    assert!(matches!(
        decode_client_binary(rejected.bytes()),
        Ok(ClientMessage::ListDocs { request_id, .. }) if request_id == "latest"
    ));
}

#[test]
fn socket_send_failure_requeues_exact_message_at_front() {
    let failed = OutboundFrame::for_test(ClientMessage::ListDocs {
        request_id: "failed".into(),
        scope_nonce: Some(1),
    });
    let mut queue = VecDeque::from([OutboundFrame::for_test(ClientMessage::Ping)]);

    requeue_front_after_send_failure(&mut queue, failed);

    assert!(matches!(
        queue.front().map(|frame| decode_client_binary(frame.bytes())),
        Some(Ok(ClientMessage::ListDocs { request_id, .. })) if request_id == "failed"
    ));
    assert!(matches!(
        queue.back().map(OutboundFrame::message_class),
        Some(super::OutboundMessageClass::Keepalive)
    ));
}

#[test]
fn prepare_queue_for_new_connection_keeps_reads_and_prepends_ping() {
    let mut queue = VecDeque::from([
        OutboundFrame::for_test(ClientMessage::StageFile {
            target: ScPathTarget::from_path("note.md"),
            scope_nonce: Some(1),
        }),
        OutboundFrame::for_test(ClientMessage::ListDocs {
            request_id: "docs".into(),
            scope_nonce: Some(1),
        }),
        OutboundFrame::for_test(ClientMessage::GetChanges {
            request_id: "changes".into(),
            scope_nonce: Some(1),
        }),
    ]);

    prepare_queue_for_new_connection(&mut queue);

    assert!(matches!(
        queue.front().map(OutboundFrame::message_class),
        Some(super::OutboundMessageClass::Keepalive)
    ));
    assert_eq!(queue.len(), 3);
    assert!(
        queue
            .iter()
            .all(|frame| frame.message_class() != super::OutboundMessageClass::Write)
    );
}

#[test]
fn output_write_classification_distinguishes_reads_from_writes() {
    assert!(is_write_message(&ClientMessage::DocumentCreate(
        DocumentCreateRequest {
            proposed_node_id: NodeId::new(),
            repo_id: RepoId::new_v4(),
            branch: None,
            scope_nonce: ScopeNonce::new(1),
            path: "Untitled.md".into(),
        }
    )));
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
        waterline: 0_u64.into(),
        server_vector: VersionVector::new(),
        snapshot_kind: None,
        source_proof: None,
        payload: vec![],
    }));
    assert!(is_write_message(&ClientMessage::MergePeer {
        peer_id: "remote-peer".into(),
        doc_id: DocId::from_u128(7),
        scope_nonce: Some(1),
    }));
    assert!(is_write_message(&ClientMessage::RepoControl(
        RepoControlRequest::SubmitLifecycle {
            request_id: uuid::Uuid::new_v4(),
            lifecycle_intent: RepoLifecycleIntent::Create {
                initial_alias: "new-repo".into(),
                current_scope_nonce: ScopeNonce::new(1),
                switch_nonce: SwitchNonce::new(2),
            },
        }
    )));
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
