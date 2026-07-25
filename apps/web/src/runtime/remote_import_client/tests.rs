//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 07_network#remote-import-wire-contract
//!   - 09_web_thin_client_ledger#remote-import-client-contract
//!
//! Regression coverage for exact Remote Import correlation and client state.

use super::*;
use crate::api::ConnectionStatus;
use crate::runtime::domain::{PendingBranchSwitch, PendingRepoSwitch};
use deve_core::models::RepoId;
use deve_core::protocol::{
    RemoteImportApplyReceipt, RemoteImportBlocker, RemoteImportCandidatePage,
    RemoteImportCandidateRevision, RemoteImportCandidateView, RemoteImportChangeKind,
    RemoteImportPageCursor, RemoteImportProjectionOutcome, RemoteImportResponseContext,
    RemoteImportSessionView, RemoteImportState, ScopeNonce, ServerError, ServerErrorCode,
};
use leptos::reactive::owner::Owner;

struct Fixture {
    _owner: Owner,
    client: RemoteImportClient,
    set_repo: WriteSignal<Option<String>>,
    set_scope: WriteSignal<u64>,
}

fn fixture() -> Fixture {
    let owner = Owner::new();
    owner.set();
    let ws = WsService::new_for_test(ConnectionStatus::Connected);
    let (repo, set_repo) = signal(Some(RepoId::new_v4().to_string()));
    let (branch, _) = signal(None::<PeerId>);
    let (scope, set_scope) = signal(7u64);
    let (pending_branch, _) = signal(None::<PendingBranchSwitch>);
    let (pending_repo, _) = signal(None::<PendingRepoSwitch>);
    let client = RemoteImportClient::new(
        ws.clone(),
        repo,
        branch,
        scope,
        pending_branch,
        pending_repo,
    );
    Fixture {
        _owner: owner,
        client,
        set_repo,
        set_scope,
    }
}

#[test]
fn projection_signal_survives_responsive_view_owner_cleanup() {
    let fixture = fixture();
    let responsive_view = fixture._owner.child();
    let projection = responsive_view.with(|| fixture.client.projection());

    responsive_view.cleanup();

    assert_eq!(
        projection.get_untracked().availability,
        RemoteImportAvailability::Offline
    );
}

fn response_context(
    request: &RemoteImportRequest,
    session_id: Option<RemoteImportSessionId>,
    revision: Option<RemoteImportCandidateRevision>,
) -> RemoteImportResponseContext {
    let context = request.context();
    RemoteImportResponseContext {
        request_id: context.request_id,
        repo_id: context.repo_id,
        branch: context.branch.clone(),
        scope_nonce: context.scope_nonce,
        session_id,
        revision,
    }
}

fn ready_session(
    session_id: RemoteImportSessionId,
    revision: RemoteImportCandidateRevision,
) -> RemoteImportSessionView {
    RemoteImportSessionView {
        session_id,
        state: RemoteImportState::Ready,
        revision: Some(revision),
        entry_count: 2,
        blockers: vec![RemoteImportBlocker::PendingOverlap],
        cleanup_pending: false,
        projection_outcome: None,
    }
}

fn clean_ready_session(
    session_id: RemoteImportSessionId,
    revision: RemoteImportCandidateRevision,
) -> RemoteImportSessionView {
    let mut session = ready_session(session_id, revision);
    session.blockers.clear();
    session
}

fn take_remote_request(fixture: &Fixture) -> RemoteImportRequest {
    let request = fixture
        .client
        .ws
        .drain_sent_for_test()
        .pop()
        .expect("request");
    let ClientMessage::RemoteImport(request) = request else {
        panic!("expected Remote Import request");
    };
    request
}

fn take_page_request(fixture: &Fixture) -> RemoteImportRequest {
    fixture
        .client
        .ws
        .drain_sent_for_test()
        .into_iter()
        .find_map(|message| match message {
            ClientMessage::RemoteImport(request @ RemoteImportRequest::Page { .. }) => {
                Some(request)
            }
            _ => None,
        })
        .expect("page request")
}

fn candidate(label: &str) -> RemoteImportCandidateView {
    RemoteImportCandidateView {
        entry_id: RemoteImportEntryId::new(format!("entry-{label}")),
        display_label: label.to_string(),
        change_kind: RemoteImportChangeKind::Added,
        blockers: Vec::new(),
    }
}

#[test]
fn list_response_requires_exact_request_and_scope() {
    let fixture = fixture();
    fixture.client.list().expect("list request");
    let request = take_remote_request(&fixture);
    let mut stale = response_context(&request, None, None);
    stale.scope_nonce = ScopeNonce::new(6);
    assert!(!fixture.client.accept(RemoteImportResponse::Sessions {
        context: stale,
        sessions: Vec::new(),
    }));

    fixture.client.list().expect("second list request");
    let request = take_remote_request(&fixture);
    assert!(fixture.client.accept(RemoteImportResponse::Sessions {
        context: response_context(&request, None, None),
        sessions: Vec::new(),
    }));
}

#[test]
fn selected_session_revision_and_backend_blockers_are_projected_verbatim() {
    let fixture = fixture();
    let session_id = RemoteImportSessionId::new(Uuid::new_v4());
    let revision = RemoteImportCandidateRevision::new(3);
    fixture
        .client
        .show(session_id, Some(revision))
        .expect("show request");
    let request = take_remote_request(&fixture);
    let session = ready_session(session_id, revision);
    assert!(fixture.client.accept(RemoteImportResponse::Session {
        context: response_context(&request, Some(session_id), Some(revision)),
        session: session.clone(),
    }));

    let projected = fixture.client.projection().get_untracked();
    assert_eq!(projected.selected_session, Some(session));
    assert_eq!(
        projected.selected_session.expect("selected").blockers,
        vec![RemoteImportBlocker::PendingOverlap]
    );
}

#[test]
fn repo_or_scope_change_retires_pending_and_projection() {
    let fixture = fixture();
    fixture.client.list().expect("list request");
    fixture.client.projection.update(|projection| {
        projection.error = Some(deve_core::protocol::ServerErrorCode::RemoteImportStale);
    });

    fixture.set_scope.set(8);
    fixture.client.synchronize_current_scope();
    assert!(fixture.client.projection().get_untracked().error.is_none());
    assert!(fixture.client.pending.lock().expect("pending").is_empty());

    fixture.set_repo.set(Some(RepoId::new_v4().to_string()));
    fixture.client.synchronize_current_scope();
    assert!(
        fixture
            .client
            .projection()
            .get_untracked()
            .sessions
            .is_empty()
    );
}

#[test]
fn exact_absence_rejects_success_and_error_with_a_revision() {
    let fixture = fixture();
    let session_id = RemoteImportSessionId::new(Uuid::new_v4());
    let revision = RemoteImportCandidateRevision::new(3);

    fixture
        .client
        .show(session_id, None)
        .expect("pre-candidate show");
    let request = take_remote_request(&fixture);
    assert!(!fixture.client.accept(RemoteImportResponse::Session {
        context: response_context(&request, Some(session_id), Some(revision)),
        session: clean_ready_session(session_id, revision),
    }));

    fixture
        .client
        .discard(session_id, None)
        .expect("pre-candidate discard");
    let request = take_remote_request(&fixture);
    assert!(!fixture.client.accept(RemoteImportResponse::Error {
        context: response_context(&request, Some(session_id), Some(revision)),
        error: ServerError::with_detail(
            ServerErrorCode::RemoteImportStale,
            "CANARY_PRIVATE_BACKEND_DETAIL",
        ),
    }));
    assert!(fixture.client.projection().get_untracked().error.is_none());
}

#[test]
fn a_late_session_response_cannot_replace_the_new_selection() {
    let fixture = fixture();
    let revision = RemoteImportCandidateRevision::new(3);
    let first = RemoteImportSessionId::new(Uuid::new_v4());
    let second = RemoteImportSessionId::new(Uuid::new_v4());

    fixture
        .client
        .show(first, Some(revision))
        .expect("first selection");
    let first_request = take_remote_request(&fixture);
    fixture
        .client
        .show(second, Some(revision))
        .expect("second selection");
    let second_request = take_remote_request(&fixture);

    assert!(!fixture.client.accept(RemoteImportResponse::Session {
        context: response_context(&first_request, Some(first), Some(revision)),
        session: clean_ready_session(first, revision),
    }));
    assert!(fixture.client.accept(RemoteImportResponse::Session {
        context: response_context(&second_request, Some(second), Some(revision)),
        session: clean_ready_session(second, revision),
    }));
    assert_eq!(
        fixture
            .client
            .projection()
            .get_untracked()
            .selected_session
            .expect("second selected")
            .session_id,
        second
    );
}

#[test]
fn reconnect_epoch_retires_projection_and_allows_a_fresh_list() {
    let mut fixture = fixture();
    fixture.client.list().expect("initial list");
    fixture.client.projection.update(|projection| {
        projection.error = Some(ServerErrorCode::RemoteImportStale);
    });

    fixture.client.ws = WsService::new_with_incoming_for_test(
        ConnectionStatus::Connected,
        2,
        std::collections::VecDeque::new(),
    );
    assert!(fixture.client.synchronize_current_scope().is_some());
    let projection = fixture.client.projection().get_untracked();
    assert!(projection.error.is_none());
    assert!(!projection.pending.any());

    fixture.client.list().expect("fresh list");
    assert!(matches!(
        take_remote_request(&fixture),
        RemoteImportRequest::List { .. }
    ));
}

#[test]
fn first_page_replaces_and_next_page_is_single_flight() {
    let fixture = fixture();
    let session_id = RemoteImportSessionId::new(Uuid::new_v4());
    let revision = RemoteImportCandidateRevision::new(3);
    let next_cursor = RemoteImportPageCursor::new("next");

    for _ in 0..2 {
        fixture
            .client
            .show(session_id, Some(revision))
            .expect("selection");
        fixture
            .client
            .first_page(session_id, revision)
            .expect("first page");
        let request = take_page_request(&fixture);
        assert!(fixture.client.accept(RemoteImportResponse::Page {
            context: response_context(&request, Some(session_id), Some(revision)),
            page: RemoteImportCandidatePage {
                session: clean_ready_session(session_id, revision),
                entries: vec![candidate("notes/a.md")],
                next_cursor: Some(next_cursor.clone()),
            },
        }));
        assert_eq!(fixture.client.projection().get_untracked().entries.len(), 1);
    }

    assert!(fixture.client.next_page().is_some());
    assert!(
        fixture.client.next_page().is_none(),
        "same cursor must have one in-flight request"
    );
}

#[test]
fn refresh_retires_old_revision_page_response() {
    let fixture = fixture();
    let session_id = RemoteImportSessionId::new(Uuid::new_v4());
    let old_revision = RemoteImportCandidateRevision::new(3);
    let new_revision = RemoteImportCandidateRevision::new(4);
    fixture
        .client
        .show(session_id, Some(old_revision))
        .expect("selection");
    let _ = take_remote_request(&fixture);
    fixture
        .client
        .first_page(session_id, old_revision)
        .expect("old page");
    let page_request = take_page_request(&fixture);
    fixture
        .client
        .refresh(session_id, old_revision)
        .expect("refresh");
    let refresh_request = take_remote_request(&fixture);

    assert!(fixture.client.accept(RemoteImportResponse::Session {
        context: response_context(&refresh_request, Some(session_id), Some(new_revision)),
        session: clean_ready_session(session_id, new_revision),
    }));
    assert!(!fixture.client.accept(RemoteImportResponse::Page {
        context: response_context(&page_request, Some(session_id), Some(old_revision)),
        page: RemoteImportCandidatePage {
            session: clean_ready_session(session_id, old_revision),
            entries: vec![candidate("stale.md")],
            next_cursor: None,
        },
    }));
}

#[test]
fn apply_receipt_is_exact_disables_repeat_and_requests_backend_state() {
    let fixture = fixture();
    let session_id = RemoteImportSessionId::new(Uuid::new_v4());
    let revision = RemoteImportCandidateRevision::new(3);
    fixture
        .client
        .show(session_id, Some(revision))
        .expect("selection");
    let show_request = take_remote_request(&fixture);
    assert!(fixture.client.accept(RemoteImportResponse::Session {
        context: response_context(&show_request, Some(session_id), Some(revision)),
        session: clean_ready_session(session_id, revision),
    }));

    fixture.client.apply(session_id, revision).expect("apply");
    assert!(
        fixture.client.apply(session_id, revision).is_none(),
        "Apply must be single-flight"
    );
    let request = take_remote_request(&fixture);
    let context = response_context(&request, Some(session_id), Some(revision));
    assert!(fixture.client.accept(RemoteImportResponse::Applied {
        context: context.clone(),
        receipt: RemoteImportApplyReceipt {
            request_id: context.request_id,
            session_id,
            revision,
            projection_outcome: RemoteImportProjectionOutcome::Pending,
        },
    }));

    let projection = fixture.client.projection().get_untracked();
    assert!(projection.selected_apply_completed());
    assert!(
        projection
            .apply_receipt_for(session_id, Some(revision))
            .is_some()
    );
    assert!(matches!(
        take_remote_request(&fixture),
        RemoteImportRequest::Show {
            session_id: actual,
            revision: Some(actual_revision),
            ..
        } if actual == session_id && actual_revision == revision
    ));
}

#[test]
fn reconnect_projects_durable_apply_outcome_from_backend_session() {
    let fixture = fixture();
    let session_id = RemoteImportSessionId::new(Uuid::new_v4());
    let revision = RemoteImportCandidateRevision::new(4);
    fixture
        .client
        .show(session_id, Some(revision))
        .expect("selection");
    let show_request = take_remote_request(&fixture);
    let mut applied = clean_ready_session(session_id, revision);
    applied.state = RemoteImportState::Applied;
    applied.projection_outcome = Some(RemoteImportProjectionOutcome::Written);
    assert!(fixture.client.accept(RemoteImportResponse::Session {
        context: response_context(&show_request, Some(session_id), Some(revision)),
        session: applied,
    }));

    let projection = fixture.client.projection().get_untracked();
    assert!(
        projection
            .apply_receipt_for(session_id, Some(revision))
            .is_none()
    );
    assert_eq!(
        projection.apply_outcome_for(session_id, Some(revision)),
        Some(RemoteImportProjectionOutcome::Written)
    );
}
