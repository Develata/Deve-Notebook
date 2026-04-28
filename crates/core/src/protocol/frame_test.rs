use super::*;
use crate::models::{PeerId, VersionVector};

#[test]
fn client_binary_frame_roundtrips() {
    let bytes = encode_client_binary(&ClientMessage::Ping).unwrap();
    assert!(bytes.starts_with(WS_FRAME_MAGIC));
    assert!(matches!(
        decode_client_binary(&bytes),
        Ok(ClientMessage::Ping)
    ));
}

#[test]
fn server_binary_frame_roundtrips() {
    let bytes = encode_server_binary(&ServerMessage::Pong).unwrap();
    assert!(matches!(
        decode_server_binary(&bytes),
        Ok(ServerMessage::Pong)
    ));
}

#[test]
fn binary_decode_reports_versioned_binary_format() {
    let bytes = encode_client_binary(&ClientMessage::Ping).unwrap();
    let decoded = decode_client_binary_with_format(&bytes).unwrap();

    assert_eq!(decoded.format, WsFrameFormat::VersionedBinary);
    assert!(matches!(decoded.message, ClientMessage::Ping));
}

#[test]
fn legacy_binary_without_magic_is_rejected() {
    let bytes = bincode::serialize(&ClientMessage::Ping).unwrap();
    assert!(matches!(
        decode_client_binary(&bytes),
        Err(ProtocolFrameError::Decode(_))
    ));
}

#[test]
fn json_frame_reports_versioned_text_format() {
    let frame = serde_json::to_string(&ClientFrame::current(ClientMessage::Ping)).unwrap();
    let decoded = decode_client_json_with_format(&frame).unwrap();

    assert_eq!(decoded.format, WsFrameFormat::VersionedJsonText);
    assert!(matches!(decoded.message, ClientMessage::Ping));
}

#[test]
fn legacy_json_text_remains_debug_compatible() {
    let decoded = decode_client_json_with_format(r#""Ping""#).unwrap();

    assert_eq!(decoded.format, WsFrameFormat::LegacyJsonText);
    assert!(matches!(decoded.message, ClientMessage::Ping));
}

#[test]
fn unsupported_version_is_rejected() {
    let frame = ServerFrame {
        protocol_version: WS_PROTOCOL_VERSION - 1,
        message: ServerMessage::Pong,
    };
    let bytes = encode_binary_frame(&frame).unwrap();
    assert!(matches!(
        decode_server_binary(&bytes),
        Err(ProtocolFrameError::UnsupportedVersion { .. })
    ));
}

#[test]
fn sync_vector_fields_roundtrip_in_current_binary_frame() {
    let repo_id = uuid::Uuid::new_v4();
    let peer = PeerId::new("peer-a");
    let mut vector = VersionVector::new();
    vector.update(peer.clone(), 7);

    let client = ClientMessage::SyncRequest {
        repo_id,
        known_vector: vector.clone(),
        requests: vec![(peer.clone(), (1, 8))],
    };
    let decoded_client = decode_client_binary(&encode_client_binary(&client).unwrap()).unwrap();
    match decoded_client {
        ClientMessage::SyncRequest { known_vector, .. } => {
            assert_eq!(known_vector, vector);
        }
        other => panic!("expected SyncRequest, got {other:?}"),
    }

    let server = ServerMessage::SyncPushSnapshot {
        peer_id: peer.clone(),
        repo_id,
        scope_nonce: 3,
        branch: Some(peer.clone()),
        server_vector: vector.clone(),
        ops: vec![],
    };
    let decoded_server = decode_server_binary(&encode_server_binary(&server).unwrap()).unwrap();
    match decoded_server {
        ServerMessage::SyncPushSnapshot { server_vector, .. } => {
            assert_eq!(server_vector, vector);
        }
        other => panic!("expected SyncPushSnapshot, got {other:?}"),
    }
}

#[test]
fn sync_vector_fields_default_for_legacy_json_debug_frames() {
    let repo_id = uuid::Uuid::new_v4();
    let client_request = format!(r#"{{"SyncRequest":{{"repo_id":"{repo_id}","requests":[]}}}}"#);
    match serde_json::from_str::<ClientMessage>(&client_request).unwrap() {
        ClientMessage::SyncRequest { known_vector, .. } => {
            assert_eq!(known_vector, VersionVector::new());
        }
        other => panic!("expected SyncRequest, got {other:?}"),
    }

    let peer = PeerId::new("peer-a");
    let server_snapshot = serde_json::json!({
        "SyncPushSnapshot": {
            "peer_id": peer,
            "repo_id": repo_id,
            "scope_nonce": 9,
            "branch": null,
            "ops": []
        }
    });
    match serde_json::from_value::<ServerMessage>(server_snapshot).unwrap() {
        ServerMessage::SyncPushSnapshot { server_vector, .. } => {
            assert_eq!(server_vector, VersionVector::new());
        }
        other => panic!("expected SyncPushSnapshot, got {other:?}"),
    }
}
