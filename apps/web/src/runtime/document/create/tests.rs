use super::*;
use deve_core::protocol::{
    DocumentCreateProjectionOutcome, DocumentCreateResponseContext, ServerErrorCode,
};

fn created(pending: &PendingDocumentCreate) -> DocumentCreateResponse {
    let request = pending.request();
    let doc_id = DocId(request.proposed_node_id.0);
    DocumentCreateResponse::Created {
        context: DocumentCreateResponseContext::from(&request),
        node_id: request.proposed_node_id,
        doc_id: Some(doc_id),
        path: request.path,
        projection_outcome: DocumentCreateProjectionOutcome::Written,
    }
}

#[test]
fn document_create_requires_typed_success_and_exact_projection_in_either_order() {
    let repo_id = RepoId::new_v4();
    let mut pending = PendingDocumentCreate::new(repo_id, 7, "notes/a.md".into(), true);
    let doc_id = DocId(pending.proposed_node_id().0);
    let docs = vec![(doc_id, "notes/a.md".into())];
    assert_eq!(pending.observe_docs(&docs), None);
    assert_eq!(
        pending.accept_response(&created(&pending)),
        CreateResponseDisposition::WaitingForProjection
    );
    assert_eq!(
        pending.observe_docs(&docs),
        Some(ProjectedCreate {
            doc_id,
            select: true
        })
    );
}

#[test]
fn document_create_settles_against_backend_normalized_path_not_raw_request() {
    let repo_id = RepoId::new_v4();
    let mut pending = PendingDocumentCreate::new(repo_id, 7, "notes/a".into(), true);
    let request = pending.request();
    let doc_id = DocId(request.proposed_node_id.0);
    let response = DocumentCreateResponse::Created {
        context: DocumentCreateResponseContext::from(&request),
        node_id: request.proposed_node_id,
        doc_id: Some(doc_id),
        path: "notes/a.md".into(),
        projection_outcome: DocumentCreateProjectionOutcome::Written,
    };
    assert_eq!(
        pending.accept_response(&response),
        CreateResponseDisposition::WaitingForProjection
    );
    assert_eq!(pending.observe_docs(&[(doc_id, "notes/a".into())]), None);
    assert_eq!(
        pending.observe_docs(&[(doc_id, "notes/a.md".into())]),
        Some(ProjectedCreate {
            doc_id,
            select: true
        })
    );
}

#[test]
fn document_create_internal_reconnect_replays_same_uuid_once_after_write_ready() {
    let repo_id = RepoId::new_v4();
    let mut pending = PendingDocumentCreate::new(repo_id, 7, "notes/a.md".into(), true);
    let proposed = pending.proposed_node_id();
    assert!(pending.rebind_internal_reconnect(repo_id, 7, 8));
    let replay = pending
        .take_replay_for_write_ready(repo_id, 8)
        .expect("one replay");
    assert_eq!(replay.proposed_node_id, proposed);
    assert_eq!(replay.scope_nonce.get(), 8);
    assert!(pending.take_replay_for_write_ready(repo_id, 8).is_none());
}

#[test]
fn document_create_reject_is_typed_and_scope_filtered() {
    let repo_id = RepoId::new_v4();
    let mut pending = PendingDocumentCreate::new(repo_id, 7, "notes/a.md".into(), true);
    let request = pending.request();
    let rejected = DocumentCreateResponse::Rejected {
        context: DocumentCreateResponseContext::from(&request),
        error: ServerError::with_detail(ServerErrorCode::StorageConflict, "PRIVATE_BACKEND_DETAIL"),
    };
    assert!(matches!(
        pending.accept_response(&rejected),
        CreateResponseDisposition::Rejected(ServerError {
            code: ServerErrorCode::StorageConflict,
            ..
        })
    ));
}
