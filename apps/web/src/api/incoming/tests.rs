use super::decode::{decode_binary_message, decode_text_message};
use super::handle_socket_event;
use super::push_server_message;
use crate::api::ConnectionStatus;
use crate::api::socket::{SocketEvent, SocketMessage};
use deve_core::codec;
use deve_core::models::Op;
use deve_core::protocol::ConfirmedOp;
use deve_core::protocol::ServerError;
use deve_core::protocol::ServerErrorCode;
use deve_core::protocol::ServerMessage;
use deve_core::protocol::frame::{
    WS_PROTOCOL_VERSION, encode_server_binary, encode_server_binary_with_version,
};
use leptos::prelude::GetUntracked;
use std::collections::VecDeque;

#[test]
fn first_binary_message_confirms_connection_and_enqueues() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let (status, set_status) = leptos::prelude::signal(ConnectionStatus::Disconnected);
    let (msg_seq, set_msg_seq) = leptos::prelude::signal(0u64);
    let (msg_queue, set_msg_queue) =
        leptos::prelude::signal(VecDeque::<(u64, u64, ServerMessage)>::new());
    let bytes = encode_server_binary(&ServerMessage::Pong).unwrap();
    let mut confirmed_connected = false;

    assert!(handle_socket_event(
        SocketEvent::Message(SocketMessage::Bytes(bytes)),
        &mut confirmed_connected,
        set_msg_seq,
        set_msg_queue,
        set_status,
        42,
    ));

    assert!(confirmed_connected);
    assert_eq!(status.get_untracked(), ConnectionStatus::Connected);
    assert_eq!(msg_seq.get_untracked(), 1);
    let queue = msg_queue.get_untracked();
    assert_eq!(queue.len(), 1);
    assert_eq!(
        queue.front().map(|(seq, epoch, _)| (*seq, *epoch)),
        Some((1, 42))
    );
    assert!(matches!(
        queue.front().map(|(_, _, msg)| msg),
        Some(ServerMessage::Pong)
    ));
}

#[test]
fn disposed_first_message_returns_false_without_panic() {
    let (set_status, set_msg_seq, set_msg_queue) = {
        let runtime = leptos::reactive::owner::Owner::new();
        runtime.set();
        let (_, set_status) = leptos::prelude::signal(ConnectionStatus::Disconnected);
        let (_, set_msg_seq) = leptos::prelude::signal(0u64);
        let (_, set_msg_queue) =
            leptos::prelude::signal(VecDeque::<(u64, u64, ServerMessage)>::new());
        (set_status, set_msg_seq, set_msg_queue)
    };
    let bytes = encode_server_binary(&ServerMessage::Pong).unwrap();
    let mut confirmed_connected = false;

    assert!(!handle_socket_event(
        SocketEvent::Message(SocketMessage::Bytes(bytes)),
        &mut confirmed_connected,
        set_msg_seq,
        set_msg_queue,
        set_status,
        42,
    ));
    assert!(!confirmed_connected);
}

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
fn binary_raw_codec_payload_is_rejected() {
    let bytes = codec::encode(&ServerMessage::Pong).unwrap();
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
fn binary_unsupported_server_version_surfaces_protocol_error() {
    let bytes =
        encode_server_binary_with_version(&ServerMessage::Pong, WS_PROTOCOL_VERSION + 1).unwrap();

    match decode_binary_message(&bytes) {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::SyncVersionMismatch);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| { detail.contains("unsupported WS protocol version") })
            );
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
}

#[test]
fn binary_malformed_versioned_payload_surfaces_protocol_error() {
    let mut bytes = encode_server_binary(&ServerMessage::Pong).unwrap();
    bytes.pop();

    match decode_binary_message(&bytes) {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::SyncInvalidPayload);
            assert_eq!(error.detail.as_deref(), Some("Invalid WS server frame"));
        }
        other => panic!("expected malformed payload ProtocolError, got {other:?}"),
    }
}

#[test]
fn text_legacy_json_still_decodes_server_message() {
    assert!(matches!(
        decode_text_message(r#""Pong""#),
        Some(ServerMessage::Pong)
    ));
}

#[test]
fn text_unsupported_server_version_surfaces_protocol_error() {
    let text = serde_json::json!({
        "protocol_version": WS_PROTOCOL_VERSION + 1,
        "message": "Pong"
    })
    .to_string();

    match decode_text_message(&text) {
        Some(ServerMessage::ProtocolError { error, .. }) => {
            assert_eq!(error.code, ServerErrorCode::SyncVersionMismatch);
            assert!(
                error
                    .detail
                    .as_deref()
                    .is_some_and(|detail| { detail.contains("unsupported WS protocol version") })
            );
        }
        other => panic!("expected ProtocolError, got {other:?}"),
    }
}

#[test]
fn binary_raw_codec_history_payload_is_rejected() {
    let bytes = codec::encode(&ServerMessage::History {
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
fn binary_raw_codec_protocol_error_payload_is_rejected() {
    let bytes = codec::encode(&ServerMessage::ProtocolError {
        error: ServerError::new(ServerErrorCode::ScCommitDiffUnprojectable),
        switch_nonce: None,
        scope_nonce: Some(7),
    })
    .unwrap();
    assert!(decode_binary_message(&bytes).is_none());
}
