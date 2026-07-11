use super::*;
use crate::models::{PeerFactSeq, PeerId, VersionVector};
use crate::protocol::{SyncPayloadKind, SyncPushHeader};

#[test]
fn sync_transfer_json_uses_plan_field_names() {
    let repo_id = uuid::Uuid::new_v4();
    let peer = PeerId::new("peer-a");

    let client = ClientMessage::SyncPush {
        source_peer_id: peer.clone(),
        repo_id,
        range_start: 1_u64.into(),
        range_end: 1_u64.into(),
        header: SyncPushHeader::diff(repo_id, peer.clone(), VersionVector::new()),
        encrypted_payload: vec![],
    };
    let client_value = serde_json::to_value(&client).unwrap();
    assert_eq!(
        client_value["SyncPush"]["source_peer_id"],
        serde_json::to_value(&peer).unwrap()
    );
    assert_eq!(
        client_value["SyncPush"]["header"]["repo_id"],
        serde_json::to_value(repo_id).unwrap()
    );
    assert_eq!(
        client_value["SyncPush"]["header"]["peer_id"],
        serde_json::to_value(&peer).unwrap()
    );
    assert_eq!(client_value["SyncPush"]["header"]["payload_kind"], "diff");
    assert!(client_value["SyncPush"].get("peer_id").is_none());
    assert!(client_value["SyncPush"].get("ops").is_none());
    assert!(client_value["SyncPush"]["encrypted_payload"].is_array());

    let client_push_plan_json = serde_json::json!({
        "SyncPush": {
            "source_peer_id": peer.clone(),
            "repo_id": repo_id,
            "range_start": 1,
            "range_end": 1,
            "header": {
                "repo_id": repo_id,
                "peer_id": peer.clone(),
                "vector": VersionVector::new(),
                "payload_kind": "diff"
            },
            "encrypted_payload": []
        }
    });
    match serde_json::from_value::<ClientMessage>(client_push_plan_json).unwrap() {
        ClientMessage::SyncPush {
            source_peer_id,
            header,
            encrypted_payload,
            ..
        } => {
            assert_eq!(source_peer_id, peer);
            assert_eq!(header.payload_kind, SyncPayloadKind::Diff);
            assert!(encrypted_payload.is_empty());
        }
        other => panic!("expected SyncPush, got {other:?}"),
    }

    let client_snapshot = ClientMessage::SyncPushSnapshot {
        source_peer_id: peer.clone(),
        repo_id,
        waterline: PeerFactSeq::ZERO,
        server_vector: VersionVector::new(),
        snapshot_kind: Some("full".to_string()),
        source_proof: None,
        payload: vec![],
    };
    let client_snapshot_value = serde_json::to_value(&client_snapshot).unwrap();
    assert_eq!(
        client_snapshot_value["SyncPushSnapshot"]["source_peer_id"],
        serde_json::to_value(&peer).unwrap()
    );
    assert!(
        client_snapshot_value["SyncPushSnapshot"]
            .get("encrypted_payload")
            .is_none()
    );
    assert!(client_snapshot_value["SyncPushSnapshot"]["payload"].is_array());

    let client_snapshot_plan_json = serde_json::json!({
        "SyncPushSnapshot": {
            "source_peer_id": peer.clone(),
            "repo_id": repo_id,
            "waterline": 0,
            "server_vector": VersionVector::new(),
            "snapshot_kind": "full",
            "payload": []
        }
    });
    match serde_json::from_value::<ClientMessage>(client_snapshot_plan_json).unwrap() {
        ClientMessage::SyncPushSnapshot {
            source_peer_id,
            server_vector,
            snapshot_kind,
            payload,
            ..
        } => {
            assert_eq!(source_peer_id, peer);
            assert_eq!(server_vector, VersionVector::new());
            assert_eq!(snapshot_kind.as_deref(), Some("full"));
            assert!(payload.is_empty());
        }
        other => panic!("expected SyncPushSnapshot, got {other:?}"),
    }

    let server = ServerMessage::SyncPushSnapshot {
        source_peer_id: peer.clone(),
        repo_id,
        waterline: PeerFactSeq::ZERO,
        scope_nonce: crate::protocol::ScopeNonce::new(9),
        branch: None,
        server_vector: VersionVector::new(),
        snapshot_kind: Some("full".to_string()),
        source_proof: None,
        payload: vec![],
    };
    let server_value = serde_json::to_value(&server).unwrap();
    assert_eq!(
        server_value["SyncPushSnapshot"]["source_peer_id"],
        serde_json::to_value(&peer).unwrap()
    );
    assert!(server_value["SyncPushSnapshot"].get("peer_id").is_none());
    assert!(server_value["SyncPushSnapshot"].get("ops").is_none());
    assert!(
        server_value["SyncPushSnapshot"]
            .get("encrypted_payload")
            .is_none()
    );
    assert!(server_value["SyncPushSnapshot"]["payload"].is_array());
}

#[test]
fn sync_transfer_json_rejects_v11_transfer_shapes() {
    let repo_id = uuid::Uuid::new_v4();
    let peer = PeerId::new("peer-a");

    let client_push_v11 = serde_json::json!({
        "SyncPush": {
            "peer_id": peer,
            "repo_id": repo_id,
            "header": {
                "repo_id": repo_id,
                "peer_id": peer,
                "vector": VersionVector::new(),
                "payload_kind": "diff"
            },
            "ops": []
        }
    });
    assert!(serde_json::from_value::<ClientMessage>(client_push_v11).is_err());

    let client_snapshot_v11 = serde_json::json!({
        "SyncPushSnapshot": {
            "peer_id": peer,
            "repo_id": repo_id,
            "ops": []
        }
    });
    assert!(serde_json::from_value::<ClientMessage>(client_snapshot_v11).is_err());

    let client_snapshot_without_waterline = serde_json::json!({
        "SyncPushSnapshot": {
            "source_peer_id": peer,
            "repo_id": repo_id,
            "payload": []
        }
    });
    assert!(serde_json::from_value::<ClientMessage>(client_snapshot_without_waterline).is_err());

    let server_snapshot_v11 = serde_json::json!({
        "SyncPushSnapshot": {
            "peer_id": peer,
            "repo_id": repo_id,
            "scope_nonce": 9,
            "branch": null,
            "ops": []
        }
    });
    assert!(serde_json::from_value::<ServerMessage>(server_snapshot_v11).is_err());
}
