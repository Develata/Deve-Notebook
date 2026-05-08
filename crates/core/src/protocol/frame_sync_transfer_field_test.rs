use super::*;
use crate::models::{PeerId, VersionVector};

#[test]
fn sync_transfer_json_uses_plan_field_names() {
    let repo_id = uuid::Uuid::new_v4();
    let peer = PeerId::new("peer-a");

    let client = ClientMessage::SyncPush {
        source_peer_id: peer.clone(),
        repo_id,
        encrypted_payload: vec![],
    };
    let client_value = serde_json::to_value(&client).unwrap();
    assert_eq!(
        client_value["SyncPush"]["source_peer_id"],
        serde_json::to_value(&peer).unwrap()
    );
    assert!(client_value["SyncPush"].get("peer_id").is_none());
    assert!(client_value["SyncPush"].get("ops").is_none());
    assert!(client_value["SyncPush"]["encrypted_payload"].is_array());

    let server = ServerMessage::SyncPushSnapshot {
        source_peer_id: peer.clone(),
        repo_id,
        scope_nonce: 9,
        branch: None,
        server_vector: VersionVector::new(),
        snapshot_kind: Some("full".to_string()),
        encrypted_payload: vec![],
    };
    let server_value = serde_json::to_value(&server).unwrap();
    assert_eq!(
        server_value["SyncPushSnapshot"]["source_peer_id"],
        serde_json::to_value(&peer).unwrap()
    );
    assert!(server_value["SyncPushSnapshot"].get("peer_id").is_none());
    assert!(server_value["SyncPushSnapshot"].get("ops").is_none());
    assert!(server_value["SyncPushSnapshot"]["encrypted_payload"].is_array());
}

#[test]
fn sync_transfer_json_accepts_legacy_debug_aliases() {
    let repo_id = uuid::Uuid::new_v4();
    let peer = PeerId::new("peer-a");

    let client_push = serde_json::json!({
        "SyncPush": {
            "peer_id": peer,
            "repo_id": repo_id,
            "ops": []
        }
    });
    match serde_json::from_value::<ClientMessage>(client_push).unwrap() {
        ClientMessage::SyncPush {
            source_peer_id,
            encrypted_payload,
            ..
        } => {
            assert_eq!(source_peer_id, peer);
            assert!(encrypted_payload.is_empty());
        }
        other => panic!("expected SyncPush, got {other:?}"),
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
            source_peer_id,
            encrypted_payload,
            ..
        } => {
            assert_eq!(source_peer_id, peer);
            assert!(encrypted_payload.is_empty());
        }
        other => panic!("expected SyncPushSnapshot, got {other:?}"),
    }
}
