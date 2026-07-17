//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view

use super::{ExternalChangesTargetOp, TargetMutationPayload, sc_query_url, server_error_from_body};
use deve_core::models::DocId;
use deve_core::protocol::{ServerError, ServerErrorCode};
use deve_core::source_control::{ChangeDomain, ChangeEntry, ChangeStatus};

fn entry() -> ChangeEntry {
    ChangeEntry {
        path: "notes\\a.md".into(),
        renamed_from: None,
        doc_id: Some(DocId::from_u128(9)),
        status: ChangeStatus::Modified,
        has_conflict: true,
        domain: ChangeDomain::WorkingDirectory,
        base_seq: None,
        target_seq: None,
    }
}

#[test]
fn target_payload_preserves_repo_scope_identity_and_domain() {
    let payload = TargetMutationPayload::from_entry(Some("repo-1".into()), 7, &entry());
    let json = serde_json::to_value(payload).expect("payload json");

    assert_eq!(json["scope_nonce"], 7);
    assert_eq!(json["repo_id"], "repo-1");
    assert_eq!(json["path"], "notes/a.md");
    assert_eq!(json["doc_id"], DocId::from_u128(9).to_string());
    assert_eq!(json["domain"], "WorkingDirectory");
}

#[test]
fn target_ops_use_external_change_mutation_endpoints() {
    assert_eq!(
        ExternalChangesTargetOp::Stage.endpoint(),
        "/api/sc/stage-pending"
    );
    assert_eq!(
        ExternalChangesTargetOp::Unstage.endpoint(),
        "/api/sc/unstage"
    );
    assert_eq!(
        ExternalChangesTargetOp::Discard.endpoint(),
        "/api/sc/discard-pending"
    );
}

#[test]
fn external_changes_query_url_preserves_repo_scope() {
    assert_eq!(
        sc_query_url("/api/sc/pending", Some("repo 1&x=1/雪"), 9),
        "/api/sc/pending?scope_nonce=9&repo_id=repo%201%26x%3D1%2F%E9%9B%AA"
    );
    assert_eq!(
        sc_query_url("/api/sc/staged", None, 9),
        "/api/sc/staged?scope_nonce=9"
    );
}

#[test]
fn rejected_error_preserves_only_structured_server_error() {
    let body = serde_json::to_string(&ServerError::with_detail(
        ServerErrorCode::ScPendingNotFound,
        "pending target vanished",
    ))
    .expect("server error json");

    assert_eq!(
        server_error_from_body(&body),
        Some(ServerError::with_detail(
            ServerErrorCode::ScPendingNotFound,
            "pending target vanished"
        ))
    );
    assert_eq!(server_error_from_body("plain backend failure"), None);
    assert_eq!(server_error_from_body(""), None);
}
