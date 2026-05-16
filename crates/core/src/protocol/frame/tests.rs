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
fn plugin_response_result_roundtrip_in_current_binary_frame() {
    let result = serde_json::json!({
        "type": "text",
        "content": "Error: No AI API key configured."
    });
    let server = ServerMessage::PluginResponse {
        req_id: "req-1".to_string(),
        result: Some(result.clone()),
        error: None,
    };

    let decoded = decode_server_binary(&encode_server_binary(&server).unwrap()).unwrap();

    match decoded {
        ServerMessage::PluginResponse {
            req_id,
            result: decoded_result,
            error,
        } => {
            assert_eq!(req_id, "req-1");
            assert_eq!(decoded_result, Some(result));
            assert!(error.is_none());
        }
        other => panic!("expected PluginResponse, got {other:?}"),
    }
}

#[test]
fn plugin_response_result_remains_json_debug_compatible() {
    let frame = ServerFrame::current(ServerMessage::PluginResponse {
        req_id: "req-1".to_string(),
        result: Some(serde_json::json!({"type": "text", "content": "ok"})),
        error: None,
    });
    let text = serde_json::to_string(&frame).unwrap();

    let decoded = decode_server_json(&text).unwrap();

    match decoded {
        ServerMessage::PluginResponse { result, .. } => {
            assert_eq!(
                result,
                Some(serde_json::json!({"type": "text", "content": "ok"}))
            );
        }
        other => panic!("expected PluginResponse, got {other:?}"),
    }
}

#[test]
fn binary_decode_reports_versioned_binary_format() {
    let bytes = encode_client_binary(&ClientMessage::Ping).unwrap();
    let decoded = decode_client_binary_with_format(&bytes).unwrap();

    assert_eq!(decoded.format, WsFrameFormat::VersionedBinary);
    assert!(matches!(decoded.message, ClientMessage::Ping));
}

#[test]
fn plugin_call_args_roundtrip_in_current_binary_frame() {
    let args = vec![
        serde_json::json!("req-1"),
        serde_json::json!({"current_file": "notes/a.md", "selection": null}),
        serde_json::json!([{"role": "user", "content": "ping"}]),
    ];
    let client = ClientMessage::PluginCall {
        req_id: "req-1".to_string(),
        plugin_id: "ai-chat".to_string(),
        fn_name: "chat".to_string(),
        args: args.clone(),
    };

    let decoded = decode_client_binary(&encode_client_binary(&client).unwrap()).unwrap();

    match decoded {
        ClientMessage::PluginCall {
            req_id,
            plugin_id,
            fn_name,
            args: decoded_args,
        } => {
            assert_eq!(req_id, "req-1");
            assert_eq!(plugin_id, "ai-chat");
            assert_eq!(fn_name, "chat");
            assert_eq!(decoded_args, args);
        }
        other => panic!("expected PluginCall, got {other:?}"),
    }
}

#[test]
fn plugin_call_args_remain_json_debug_compatible() {
    let frame = ClientFrame::current(ClientMessage::PluginCall {
        req_id: "req-1".to_string(),
        plugin_id: "ai-chat".to_string(),
        fn_name: "chat".to_string(),
        args: vec![serde_json::json!({"nested": ["ok"]})],
    });
    let text = serde_json::to_string(&frame).unwrap();

    let decoded = decode_client_json(&text).unwrap();

    match decoded {
        ClientMessage::PluginCall { args, .. } => {
            assert_eq!(args, vec![serde_json::json!({"nested": ["ok"]})]);
        }
        other => panic!("expected PluginCall, got {other:?}"),
    }
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
        protocol_version: MIN_SUPPORTED_WS_PROTOCOL_VERSION - 1,
        message: ServerMessage::Pong,
    };
    let bytes = encode_binary_frame(&frame).unwrap();
    assert!(matches!(
        decode_server_binary(&bytes),
        Err(ProtocolFrameError::UnsupportedVersion { .. })
    ));
}

#[test]
fn unsupported_binary_version_is_checked_before_message_schema() {
    #[derive(serde::Serialize)]
    struct LegacyClientFrame {
        protocol_version: u16,
        message: LegacyClientMessage,
    }

    #[derive(serde::Serialize)]
    enum LegacyClientMessage {
        SyncSnapshotRequest {
            peer_id: PeerId,
            repo_id: uuid::Uuid,
            known_vector: VersionVector,
        },
    }

    let bytes = encode_binary_frame(&LegacyClientFrame {
        protocol_version: WS_PROTOCOL_VERSION - 1,
        message: LegacyClientMessage::SyncSnapshotRequest {
            peer_id: PeerId::new("peer-a"),
            repo_id: uuid::Uuid::new_v4(),
            known_vector: VersionVector::new(),
        },
    })
    .unwrap();

    assert!(matches!(
        decode_client_binary(&bytes),
        Err(ProtocolFrameError::UnsupportedVersion { received, .. })
            if received == WS_PROTOCOL_VERSION - 1
    ));
}

#[test]
fn unsupported_json_version_is_checked_before_message_schema() {
    let text = serde_json::json!({
        "protocol_version": WS_PROTOCOL_VERSION - 1,
        "message": {}
    })
    .to_string();

    assert!(matches!(
        decode_client_json(&text),
        Err(ProtocolFrameError::UnsupportedVersion { received, .. })
            if received == WS_PROTOCOL_VERSION - 1
    ));
}

#[test]
fn minimum_supported_binary_version_still_decodes() {
    let client_frame = ClientFrame {
        protocol_version: MIN_SUPPORTED_WS_PROTOCOL_VERSION,
        message: ClientMessage::Ping,
    };
    let client_bytes = encode_binary_frame(&client_frame).unwrap();
    assert!(matches!(
        decode_client_binary(&client_bytes),
        Ok(ClientMessage::Ping)
    ));

    let server_frame = ServerFrame {
        protocol_version: MIN_SUPPORTED_WS_PROTOCOL_VERSION,
        message: ServerMessage::Pong,
    };
    let server_bytes = encode_binary_frame(&server_frame).unwrap();
    assert!(matches!(
        decode_server_binary(&server_bytes),
        Ok(ServerMessage::Pong)
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
        source_peer_id: peer.clone(),
        repo_id,
        scope_nonce: crate::protocol::ScopeNonce::new(3),
        branch: Some(peer.clone()),
        server_vector: vector.clone(),
        snapshot_kind: Some("full".to_string()),
        source_proof: None,
        payload: vec![],
    };
    let decoded_server = decode_server_binary(&encode_server_binary(&server).unwrap()).unwrap();
    match decoded_server {
        ServerMessage::SyncPushSnapshot {
            server_vector,
            snapshot_kind,
            ..
        } => {
            assert_eq!(server_vector, vector);
            assert_eq!(snapshot_kind.as_deref(), Some("full"));
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
    let client_snapshot_request = serde_json::json!({
        "SyncSnapshotRequest": {
            "peer_id": peer,
            "repo_id": repo_id
        }
    });
    match serde_json::from_value::<ClientMessage>(client_snapshot_request).unwrap() {
        ClientMessage::SyncSnapshotRequest {
            known_vector,
            reason,
            ..
        } => {
            assert_eq!(known_vector, VersionVector::new());
            assert_eq!(reason, None);
        }
        other => panic!("expected SyncSnapshotRequest, got {other:?}"),
    }

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
        ServerMessage::SyncPushSnapshot {
            server_vector,
            snapshot_kind,
            ..
        } => {
            assert_eq!(server_vector, VersionVector::new());
            assert_eq!(snapshot_kind, None);
        }
        other => panic!("expected SyncPushSnapshot, got {other:?}"),
    }
}
