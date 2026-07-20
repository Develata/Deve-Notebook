//! plan_ref:
//!   - 07_network#remote-import-wire-contract
//!
//! F4/v3 Remote Import wire contract tests.

use super::*;
use crate::protocol::frame::{
    ClientFrame, ServerFrame, WS_FRAME_MAGIC, WS_PROTOCOL_VERSION, decode_client_binary,
    decode_client_binary_frame, decode_client_json, decode_server_binary, decode_server_json,
    encode_client_binary, encode_server_binary,
};
use crate::protocol::{ClientMessage, ServerErrorCode, ServerMessage};
use crate::source_control::diff_projection::compute_diff_projection;

#[test]
fn write_classification_is_backend_action_exact() {
    let context = request_context();
    assert!(
        RemoteImportRequest::Prepare {
            context: context.clone(),
            provider: RemoteProjectionProvider::WebDav,
        }
        .is_write()
    );
    assert!(!RemoteImportRequest::List { context }.is_write());
}

#[test]
fn remote_import_nested_wire_roundtrips_in_f4_v3_binary_and_versioned_json() {
    for request in request_variants() {
        let message = ClientMessage::RemoteImport(request);
        let expected = serde_json::to_value(&message).expect("serialize expected client message");

        let binary = encode_client_binary(&message).expect("encode F4/v3 client frame");
        assert!(binary.starts_with(WS_FRAME_MAGIC));
        let frame = decode_client_binary_frame(&binary).expect("decode F4/v3 client frame");
        assert_eq!(frame.protocol_version, WS_PROTOCOL_VERSION);
        assert_eq!(
            serde_json::to_value(decode_client_binary(&binary).expect("decode client message"))
                .expect("serialize decoded client message"),
            expected
        );

        let json = serde_json::to_string(&ClientFrame::current(message.clone()))
            .expect("encode versioned debug client JSON");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).expect("parse client JSON")["protocol_version"],
            serde_json::json!(WS_PROTOCOL_VERSION)
        );
        assert_eq!(
            serde_json::to_value(decode_client_json(&json).expect("decode client debug JSON"))
                .expect("serialize decoded client JSON"),
            expected
        );
    }

    for response in response_variants() {
        let message = ServerMessage::RemoteImport(response);
        let expected = serde_json::to_value(&message).expect("serialize expected server message");

        let binary = encode_server_binary(&message).expect("encode F4/v3 server frame");
        assert!(binary.starts_with(WS_FRAME_MAGIC));
        assert_eq!(
            serde_json::to_value(decode_server_binary(&binary).expect("decode server message"))
                .expect("serialize decoded server message"),
            expected
        );

        let json = serde_json::to_string(&ServerFrame::current(message.clone()))
            .expect("encode versioned debug server JSON");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&json).expect("parse server JSON")["protocol_version"],
            serde_json::json!(WS_PROTOCOL_VERSION)
        );
        assert_eq!(
            serde_json::to_value(decode_server_json(&json).expect("decode server debug JSON"))
                .expect("serialize decoded server JSON"),
            expected
        );
    }
}

#[test]
fn remote_import_diff_wire_exposes_only_safe_review_projection_fields() {
    let response = ServerMessage::RemoteImport(RemoteImportResponse::Diff {
        context: response_context(),
        entry_id: RemoteImportEntryId::new("opaque-entry-id"),
        display_label: "Review item".to_string(),
        change_kind: RemoteImportChangeKind::Modified,
        blockers: vec![RemoteImportBlocker::PendingOverlap],
        projection: Arc::new(
            compute_diff_projection("before".to_string(), "after".to_string())
                .expect("typed diff projection"),
        ),
    });
    let value = serde_json::to_value(response).expect("serialize Remote Import diff response");
    let fields = value
        .get("RemoteImport")
        .and_then(|value| value.get("Diff"))
        .and_then(serde_json::Value::as_object)
        .expect("nested Remote Import Diff fields");
    let actual = fields
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let expected = [
        "blockers",
        "change_kind",
        "context",
        "display_label",
        "entry_id",
        "projection",
    ]
    .into_iter()
    .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(actual, expected);

    let context = fields
        .get("context")
        .and_then(serde_json::Value::as_object)
        .expect("response context fields");
    let context_fields = context
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    let expected_context = [
        "branch",
        "repo_id",
        "request_id",
        "revision",
        "scope_nonce",
        "session_id",
    ]
    .into_iter()
    .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(context_fields, expected_context);

    let serialized = serde_json::to_string(&value).expect("serialize safe-field canary");
    for forbidden in [
        "locator",
        "credential",
        "blob_path",
        "host_path",
        "provider_path",
        "source_manifest",
        "raw_failure_detail",
        "sha256_digest",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "Remote Import diff wire leaked forbidden field {forbidden}"
        );
    }
}

fn request_variants() -> Vec<RemoteImportRequest> {
    let context = request_context();
    let session_id = session_id();
    let revision = revision();
    vec![
        RemoteImportRequest::Prepare {
            context: context.clone(),
            provider: RemoteProjectionProvider::WebDav,
        },
        RemoteImportRequest::List {
            context: context.clone(),
        },
        RemoteImportRequest::Show {
            context: context.clone(),
            session_id,
            revision: Some(revision),
        },
        RemoteImportRequest::Show {
            context: context.clone(),
            session_id,
            revision: None,
        },
        RemoteImportRequest::Page {
            context: context.clone(),
            session_id,
            revision,
            cursor: Some(RemoteImportPageCursor::new("opaque-cursor")),
            limit: 100,
        },
        RemoteImportRequest::Diff {
            context: context.clone(),
            session_id,
            revision,
            entry_id: RemoteImportEntryId::new("opaque-entry-id"),
        },
        RemoteImportRequest::Refresh {
            context: context.clone(),
            session_id,
            revision,
        },
        RemoteImportRequest::Apply {
            context: context.clone(),
            session_id,
            revision,
        },
        RemoteImportRequest::Discard {
            context: context.clone(),
            session_id,
            revision: Some(revision),
        },
        RemoteImportRequest::Discard {
            context,
            session_id,
            revision: None,
        },
    ]
}

fn response_variants() -> Vec<RemoteImportResponse> {
    let context = response_context();
    let session = session_view();
    let page = RemoteImportCandidatePage {
        session: session.clone(),
        entries: vec![RemoteImportCandidateView {
            entry_id: RemoteImportEntryId::new("opaque-entry-id"),
            display_label: "Review item".to_string(),
            change_kind: RemoteImportChangeKind::Modified,
            blockers: vec![RemoteImportBlocker::PendingOverlap],
        }],
        next_cursor: Some(RemoteImportPageCursor::new("opaque-cursor")),
    };
    vec![
        RemoteImportResponse::Prepared {
            context: context.clone(),
            session: session.clone(),
        },
        RemoteImportResponse::Sessions {
            context: context.clone(),
            sessions: vec![session.clone()],
        },
        RemoteImportResponse::Session {
            context: context.clone(),
            session: session.clone(),
        },
        RemoteImportResponse::Page {
            context: context.clone(),
            page,
        },
        RemoteImportResponse::Diff {
            context: context.clone(),
            entry_id: RemoteImportEntryId::new("opaque-entry-id"),
            display_label: "Review item".to_string(),
            change_kind: RemoteImportChangeKind::Modified,
            blockers: vec![RemoteImportBlocker::PendingOverlap],
            projection: Arc::new(
                compute_diff_projection("before".to_string(), "after".to_string())
                    .expect("typed diff projection"),
            ),
        },
        RemoteImportResponse::Applied {
            context: context.clone(),
            receipt: RemoteImportApplyReceipt {
                request_id: context.request_id,
                session_id: session_id(),
                revision: revision(),
                projection_outcome: RemoteImportProjectionOutcome::Written,
            },
        },
        RemoteImportResponse::Discarded {
            context: context.clone(),
            session,
        },
        RemoteImportResponse::Error {
            context,
            error: ServerError::new(ServerErrorCode::RemoteImportBlocked),
        },
    ]
}

fn request_context() -> RemoteImportRequestContext {
    RemoteImportRequestContext {
        request_id: Uuid::from_u128(0x100),
        repo_id: Uuid::from_u128(0x200),
        branch: None,
        scope_nonce: ScopeNonce::new(7),
    }
}

fn response_context() -> RemoteImportResponseContext {
    let context = request_context();
    RemoteImportResponseContext {
        request_id: context.request_id,
        repo_id: context.repo_id,
        branch: context.branch,
        scope_nonce: context.scope_nonce,
        session_id: Some(session_id()),
        revision: Some(revision()),
    }
}

fn session_view() -> RemoteImportSessionView {
    RemoteImportSessionView {
        session_id: session_id(),
        state: RemoteImportState::Ready,
        revision: Some(revision()),
        entry_count: 1,
        blockers: vec![RemoteImportBlocker::PendingOverlap],
        cleanup_pending: false,
        projection_outcome: None,
    }
}

fn session_id() -> RemoteImportSessionId {
    RemoteImportSessionId::new(Uuid::from_u128(0x300))
}

fn revision() -> RemoteImportCandidateRevision {
    RemoteImportCandidateRevision::new(5)
}
