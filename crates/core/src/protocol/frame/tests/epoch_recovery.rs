use super::*;
use crate::protocol::{
    ProjectionRecoveryCause, ProjectionRecoveryPlan, ProjectionRecoveryRequired, ScopeNonce,
};
use std::sync::Arc;

#[test]
fn first_public_ws_epoch_is_lockstep() {
    assert_eq!(WS_FRAME_MAGIC, b"DEVEWSF4");
    assert_eq!(WS_PROTOCOL_VERSION, 2);
    assert_eq!(MIN_SUPPORTED_WS_PROTOCOL_VERSION, 2);

    for unsupported in [0, 1, 13] {
        let frame = ClientFrame {
            protocol_version: unsupported,
            message: ClientMessage::Ping,
        };
        let bytes = encode_binary_frame(&frame).unwrap();
        assert!(matches!(
            decode_client_binary(&bytes),
            Err(ProtocolFrameError::UnsupportedVersion { received, .. })
                if received == unsupported
        ));
    }
}

#[test]
fn historical_development_ws_namespace_is_rejected() {
    let frame = ClientFrame {
        protocol_version: 13,
        message: ClientMessage::Ping,
    };
    let body = crate::codec::encode(&frame).unwrap();
    for historical_magic in [b"DEVEWSF2".as_slice(), b"DEVEWSF3".as_slice()] {
        let mut bytes = historical_magic.to_vec();
        bytes.extend_from_slice(&body);
        assert!(matches!(
            decode_client_binary(&bytes),
            Err(ProtocolFrameError::Decode(detail)) if detail == MISSING_WS_FRAME_MAGIC
        ));
    }

    assert!(matches!(
        decode_client_binary(&body),
        Err(ProtocolFrameError::Decode(detail)) if detail == MISSING_WS_FRAME_MAGIC
    ));
}

#[test]
fn diff_projection_messages_roundtrip_in_binary_and_json() {
    let repo_id = uuid::Uuid::new_v4();
    let client = ClientMessage::ComputeDiffProjection {
        request_id: "diff-1".into(),
        revision: 4,
        base_content: "old 😀".into(),
        target_content: "new 中文".into(),
        scope_nonce: Some(9),
    };
    match decode_client_binary(&encode_client_binary(&client).unwrap()).unwrap() {
        ClientMessage::ComputeDiffProjection {
            request_id,
            revision,
            scope_nonce,
            ..
        } => {
            assert_eq!(request_id, "diff-1");
            assert_eq!(revision, 4);
            assert_eq!(scope_nonce, Some(9));
        }
        other => panic!("expected ComputeDiffProjection, got {other:?}"),
    }

    let projection = Arc::new(
        crate::source_control::diff_projection::compute_diff_projection(
            "old 😀".into(),
            "new 中文".into(),
        )
        .unwrap(),
    );
    let server = ServerMessage::DiffProjectionResult {
        request_id: "diff-1".into(),
        revision: 4,
        repo_id,
        branch: None,
        scope_nonce: crate::protocol::ScopeNonce::new(9),
        projection: projection.clone(),
    };
    let binary = decode_server_binary(&encode_server_binary(&server).unwrap()).unwrap();
    let json =
        decode_server_json(&serde_json::to_string(&ServerFrame::current(server)).unwrap()).unwrap();
    for decoded in [binary, json] {
        match decoded {
            ServerMessage::DiffProjectionResult {
                request_id,
                revision,
                projection: decoded_projection,
                ..
            } => {
                assert_eq!(request_id, "diff-1");
                assert_eq!(revision, 4);
                assert_eq!(decoded_projection.projection_id, projection.projection_id);
            }
            other => panic!("expected DiffProjectionResult, got {other:?}"),
        }
    }
}

#[test]
fn projection_recovery_and_external_apply_receipt_roundtrip() {
    let repo_id = uuid::Uuid::new_v4();
    let doc_id = crate::models::DocId::new();
    let recovery = ProjectionRecoveryRequired {
        repo_id,
        branch: None,
        scope_nonce: Some(7),
        cause: ProjectionRecoveryCause::ExternalApply,
        plan: ProjectionRecoveryPlan::external_apply(vec![doc_id]),
    };
    let decoded = decode_server_binary(
        &encode_server_binary(&ServerMessage::ProjectionRecoveryRequired(recovery.clone()))
            .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        decoded,
        ServerMessage::ProjectionRecoveryRequired(decoded) if decoded == recovery
    ));

    let receipt = crate::source_control::ExternalApplyReceipt {
        repo_id,
        authority_head: crate::models::GlobalSeq::from_storage_key(11),
        affected_docs: vec![doc_id],
        applied_target_count: 1,
    };
    let ack = ServerMessage::ExternalApplyAck {
        request_id: "apply-1".into(),
        receipt: receipt.clone(),
        repo_id,
        branch: None,
        scope_nonce: ScopeNonce::new(7),
    };
    match decode_server_binary(&encode_server_binary(&ack).unwrap()).unwrap() {
        ServerMessage::ExternalApplyAck {
            request_id,
            receipt: decoded_receipt,
            ..
        } => {
            assert_eq!(request_id, "apply-1");
            assert_eq!(decoded_receipt, receipt);
        }
        other => panic!("expected ExternalApplyAck, got {other:?}"),
    }
}
