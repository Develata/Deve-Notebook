//! plan_ref:
//!   - 07_network#repo-control-wire-contract
//!   - 09_web_thin_client_ledger#repo-control-client-contract

use super::*;
use crate::api::ConnectionStatus;
use deve_core::protocol::{ServerError, ServerErrorCode};

fn scope(epoch: u64, repo_id: Option<RepoId>, nonce: u64) -> RepoControlScope {
    RepoControlScope::new(epoch, repo_id, None, nonce)
}

#[test]
fn stale_connection_or_scope_discards_response() {
    let client = RepoControlClient::default();
    let repo_id = RepoId::new_v4();
    let request_id = Uuid::new_v4();
    client.register(
        request_id,
        scope(4, Some(repo_id), 8),
        PendingKind::Alias { repo_id },
    );

    let response = RepoControlResponse::AliasSet {
        request_id,
        binding: RepoAliasBinding {
            repo_id,
            display_alias: "local".into(),
            alias_revision: 2,
        },
    };
    assert_eq!(client.accept(response, &scope(5, Some(repo_id), 8)), None);
}

#[test]
fn lifecycle_identity_mismatch_is_fail_closed() {
    let client = RepoControlClient::default();
    let repo_id = RepoId::new_v4();
    let request_id = Uuid::new_v4();
    client.register(
        request_id,
        scope(4, Some(repo_id), 8),
        PendingKind::Lifecycle {
            lifecycle: PendingLifecycle::Remove { repo_id },
            accepted: None,
        },
    );

    assert_eq!(
        client.accept(
            RepoControlResponse::LifecycleAccepted {
                request_id,
                job_id: Uuid::new_v4(),
                target_repo_id: RepoId::new_v4(),
            },
            &scope(4, Some(repo_id), 8),
        ),
        None
    );
}

#[test]
fn error_admission_exposes_only_typed_code() {
    let client = RepoControlClient::default();
    let repo_id = RepoId::new_v4();
    let request_id = Uuid::new_v4();
    client.register(
        request_id,
        scope(4, Some(repo_id), 8),
        PendingKind::Lifecycle {
            lifecycle: PendingLifecycle::Remove { repo_id },
            accepted: None,
        },
    );

    let admission = client.accept(
        RepoControlResponse::Error {
            request_id,
            error: ServerError::with_detail(
                ServerErrorCode::RepoLifecycleBusy,
                "CANARY_PRIVATE_BACKEND_DETAIL",
            ),
        },
        &scope(4, Some(repo_id), 8),
    );
    assert_eq!(
        admission,
        Some(RepoControlAdmission::Error {
            code: ServerErrorCode::RepoLifecycleBusy,
            lifecycle_request: true,
        })
    );
}

#[test]
fn reconnect_rebinds_lifecycle_and_requests_exact_status_once() {
    let owner = leptos::reactive::owner::Owner::new();
    owner.with(|| {
        let client = RepoControlClient::default();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let repo_id = RepoId::new_v4();
        let request_id = Uuid::new_v4();
        client.register(
            request_id,
            scope(4, Some(repo_id), 8),
            PendingKind::Lifecycle {
                lifecycle: PendingLifecycle::Remove { repo_id },
                accepted: Some((Uuid::new_v4(), repo_id)),
            },
        );

        let current = scope(5, Some(repo_id), 8);
        assert_eq!(client.resume_lifecycles(&ws, current.clone()), 1);
        let sent = ws.drain_sent_for_test();
        assert_eq!(sent.len(), 1);
        match &sent[0] {
            ClientMessage::RepoControl(request) => {
                assert_eq!(request, &RepoControlRequest::GetLifecycle { request_id });
            }
            other => panic!("unexpected client message: {other:?}"),
        }
        assert_eq!(client.resume_lifecycles(&ws, current), 0);
        assert!(ws.drain_sent_for_test().is_empty());
    });
}

#[test]
fn status_can_rebind_identity_after_reconnect_and_terminal_clears_pending() {
    let client = RepoControlClient::default();
    let repo_id = RepoId::new_v4();
    let request_id = Uuid::new_v4();
    let job_id = Uuid::new_v4();
    let current = scope(5, Some(repo_id), 8);
    client.register(
        request_id,
        current.clone(),
        PendingKind::Lifecycle {
            lifecycle: PendingLifecycle::Remove { repo_id },
            accepted: None,
        },
    );

    assert_eq!(
        client.accept(
            RepoControlResponse::LifecycleStatus {
                request_id,
                job_id,
                target_repo_id: repo_id,
                operation: RepoLifecycleOperation::Remove,
                state: RepoLifecycleState::Terminal,
                outcome: Some(RepoLifecycleOutcome::NotCommitted),
                publication_pending: false,
            },
            &current,
        ),
        Some(RepoControlAdmission::LifecycleStatus {
            request_id,
            job_id,
            target_repo_id: repo_id,
            operation: RepoLifecycleOperation::Remove,
            state: RepoLifecycleState::Terminal,
            outcome: Some(RepoLifecycleOutcome::NotCommitted),
            publication_pending: false,
        })
    );
    assert_eq!(
        client.accept(
            RepoControlResponse::LifecycleAccepted {
                request_id,
                job_id,
                target_repo_id: repo_id,
            },
            &current,
        ),
        None
    );
}
