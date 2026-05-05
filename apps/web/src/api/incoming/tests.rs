use super::decode::{decode_binary_message, decode_text_message};
use super::push_server_message;
use deve_core::models::Op;
use deve_core::protocol::ConfirmedOp;
use deve_core::protocol::ServerError;
use deve_core::protocol::ServerErrorCode;
use deve_core::protocol::ServerMessage;
use deve_core::protocol::frame::encode_server_binary;
use std::collections::VecDeque;

#[test]
fn incoming_queue_keeps_latest_messages_in_order() {
    let mut queue = VecDeque::new();
    for seq in 1..=300 {
        push_server_message(&mut queue, seq, 7, ServerMessage::Pong);
    }

    assert_eq!(queue.len(), 256);
    assert_eq!(queue.front().map(|(seq, _, _)| *seq), Some(45));
    assert_eq!(queue.back().map(|(seq, _, _)| *seq), Some(300));
    assert_eq!(queue.front().map(|(_, epoch, _)| *epoch), Some(7));
}

#[test]
fn binary_json_legacy_payload_is_rejected() {
    let bytes = br#""Pong""#;
    assert!(decode_binary_message(bytes).is_none());
}

#[test]
fn binary_bincode_legacy_payload_is_rejected() {
    let bytes = bincode::serialize(&ServerMessage::Pong).unwrap();
    assert!(decode_binary_message(&bytes).is_none());
}

#[test]
fn binary_versioned_frame_decodes_server_message() {
    let bytes = encode_server_binary(&ServerMessage::Pong).unwrap();
    assert!(matches!(
        decode_binary_message(&bytes),
        Some(ServerMessage::Pong)
    ));
}

#[test]
fn text_legacy_json_still_decodes_server_message() {
    assert!(matches!(
        decode_text_message(r#""Pong""#),
        Some(ServerMessage::Pong)
    ));
}

#[test]
fn binary_bincode_history_legacy_payload_is_rejected() {
    let bytes = bincode::serialize(&ServerMessage::History {
        repo_id: uuid::Uuid::nil(),
        branch: None,
        scope_nonce: Some(1),
        doc_id: deve_core::models::DocId::from_u128(7),
        request_id: 9,
        ops: vec![ConfirmedOp::new(
            3,
            Op::Insert {
                pos: 0,
                content: "hello".into(),
            },
            None,
        )],
    })
    .unwrap();
    assert!(decode_binary_message(&bytes).is_none());
}

#[test]
fn binary_bincode_protocol_error_legacy_payload_is_rejected() {
    let bytes = bincode::serialize(&ServerMessage::ProtocolError {
        error: ServerError::new(ServerErrorCode::ScCommitDiffUnprojectable),
        switch_nonce: None,
        scope_nonce: Some(7),
    })
    .unwrap();
    assert!(decode_binary_message(&bytes).is_none());
}
