//! plan_ref:
//!   - 07_network#repo-control-wire-contract
//!   - 09_web_thin_client_ledger#repo-control-client-contract

use super::*;
use crate::api::ConnectionStatus;
use deve_core::protocol::{
    LocalRepoRemovalDeletedCategory, LocalRepoRemovalPreservedCategory, LocalRepoRemovalWarning,
    ServerError, ServerErrorCode,
};

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
fn removal_preview_accepts_an_exact_non_current_repo_target() {
    let client = RepoControlClient::default();
    let current_repo_id = RepoId::new_v4();
    let removed_repo_id = RepoId::new_v4();
    let request_id = Uuid::new_v4();
    let current = scope(4, Some(current_repo_id), 8);
    client.register(
        request_id,
        current.clone(),
        PendingKind::RemovalPrepare {
            repo_id: removed_repo_id,
            display_alias: "archive".into(),
        },
    );

    let admission = client.accept(
        RepoControlResponse::LocalRepoRemovalPrepared {
            request_id,
            preparation_id: Uuid::new_v4(),
            repo_id: removed_repo_id,
            preview: LocalRepoRemovalPreview {
                deleted: Vec::new(),
                preserved: Vec::new(),
                warnings: Vec::new(),
                blockers: Vec::new(),
            },
            confirmation_token: RemovalConfirmationToken::from_backend("a".repeat(64)),
            fallback_binding: None,
            expires_at_unix_ms: Some(123),
        },
        &current,
    );

    assert!(matches!(
        admission,
        Some(RepoControlAdmission::RemovalPrepared { presentation })
            if presentation.repo_id == removed_repo_id
    ));
}

#[test]
fn prepared_removal_consumes_secret_once_and_accepts_one_typed_finalization() {
    let owner = leptos::reactive::owner::Owner::new();
    owner.with(|| {
        let client = RepoControlClient::default();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let repo_id = RepoId::new_v4();
        let current = scope(4, Some(repo_id), 8);
        let prepare_request = client.prepare_remove_repo(
            &ws,
            current.clone(),
            repo_id,
            "research".into(),
        );
        ws.drain_sent_for_test();
        let preparation_id = Uuid::new_v4();
        assert!(matches!(
            client.accept(
                RepoControlResponse::LocalRepoRemovalPrepared {
                    request_id: prepare_request,
                    preparation_id,
                    repo_id,
                    preview: LocalRepoRemovalPreview {
                        deleted: vec![LocalRepoRemovalDeletedCategory::LocalLedgerAuthority],
                        preserved: vec![LocalRepoRemovalPreservedCategory::WorkspaceContent],
                        warnings: vec![LocalRepoRemovalWarning::LedgerHistoryHasNoSupportedRestore],
                        blockers: Vec::new(),
                    },
                    confirmation_token: RemovalConfirmationToken::from_backend("a".repeat(64)),
                    fallback_binding: None,
                    expires_at_unix_ms: Some(123),
                },
                &current,
            ),
            Some(RepoControlAdmission::RemovalPrepared { presentation })
                if presentation.can_execute
        ));

        let execute_request = client
            .execute_prepared_removal(&ws, current.clone(), repo_id, 9)
            .expect("execute prepared removal");
        assert_eq!(
            client.execute_prepared_removal(&ws, current.clone(), repo_id, 10),
            Err(PreparedRemovalExecutionError::Missing)
        );
        let sent = ws.drain_sent_for_test();
        assert_eq!(sent.len(), 1);
        assert!(matches!(
            &sent[0],
            ClientMessage::RepoControl(RepoControlRequest::ExecuteLocalRepoRemoval {
                request_id,
                preparation_id: actual_preparation,
                current_scope_nonce,
                switch_nonce,
                ..
            }) if *request_id == execute_request
                && *actual_preparation == preparation_id
                && current_scope_nonce.get() == 8
                && switch_nonce.get() == 9
        ));

        let job_id = Uuid::new_v4();
        assert!(matches!(
            client.accept(
                RepoControlResponse::LifecycleAccepted {
                    request_id: execute_request,
                    job_id,
                    target_repo_id: repo_id,
                },
                &current,
            ),
            Some(RepoControlAdmission::LifecycleAccepted { .. })
        ));
        let finalization = client.accept(
            RepoControlResponse::LocalRepoRemovalSettled {
                request_id: execute_request,
                job_id,
                removed_repo_id: repo_id,
                final_repo_list: Vec::new(),
                scope: RepoRemovalFinalScope::NoScope {
                    scope_nonce: ScopeNonce::new(9),
                },
            },
            &current,
        );
        assert!(matches!(
            finalization,
            Some(RepoControlAdmission::RemovalFinalized {
                request_id: Some(request_id),
                job_id: actual_job,
                removed_repo_id,
                ..
            }) if request_id == execute_request && actual_job == job_id && removed_repo_id == repo_id
        ));
    });
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
            removal_request: true,
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
fn same_connection_scope_change_rebinds_lifecycle_once() {
    let owner = leptos::reactive::owner::Owner::new();
    owner.with(|| {
        let client = RepoControlClient::default();
        let ws = WsService::new_for_test(ConnectionStatus::Connected);
        let removed_repo_id = RepoId::new_v4();
        let selected_repo_id = RepoId::new_v4();
        let request_id = Uuid::new_v4();
        client.register(
            request_id,
            scope(4, Some(removed_repo_id), 8),
            PendingKind::Lifecycle {
                lifecycle: PendingLifecycle::Remove {
                    repo_id: removed_repo_id,
                },
                accepted: Some((Uuid::new_v4(), removed_repo_id)),
            },
        );

        let selected = scope(4, Some(selected_repo_id), 9);
        assert_eq!(client.resume_lifecycles(&ws, selected.clone()), 1);
        let sent = ws.drain_sent_for_test();
        assert_eq!(sent.len(), 1);
        assert!(matches!(
            &sent[0],
            ClientMessage::RepoControl(RepoControlRequest::GetLifecycle {
                request_id: actual_request_id,
            }) if *actual_request_id == request_id
        ));
        assert_eq!(client.resume_lifecycles(&ws, selected), 0);
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
