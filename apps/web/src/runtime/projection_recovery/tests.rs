use super::*;
use deve_core::models::{DocId, RepoId};
use deve_core::protocol::{
    ProjectionRecoveryCause, ProjectionRecoveryPlan, ProjectionRecoveryRequired,
};

fn required(
    repo_id: RepoId,
    doc_id: DocId,
    cause: ProjectionRecoveryCause,
) -> ProjectionRecoveryRequired {
    ProjectionRecoveryRequired {
        repo_id,
        branch: None,
        scope_nonce: Some(7),
        cause,
        plan: ProjectionRecoveryPlan::external_apply(vec![doc_id]),
    }
}

#[test]
fn projection_recovery_exact_scope_only_reopens_affected_current_document() {
    let repo_id = RepoId::new_v4();
    let current_doc = DocId::new();
    let other_doc = DocId::new();
    let scope = ProjectionRecoveryScope {
        repo_id: Some(repo_id),
        branch: None,
        scope_nonce: 7,
        current_doc: Some(current_doc),
        scope_switch_pending: false,
    };

    let current = evaluate_recovery(
        &required(repo_id, current_doc, ProjectionRecoveryCause::ExternalApply),
        &scope,
    )
    .expect("matching scope");
    assert!(current.current_document_affected);

    let unrelated = evaluate_recovery(
        &required(repo_id, other_doc, ProjectionRecoveryCause::ExternalApply),
        &scope,
    )
    .expect("matching scope");
    assert!(!unrelated.current_document_affected);
}

#[test]
fn projection_recovery_rejects_stale_scope_nonce() {
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::new();
    let scope = ProjectionRecoveryScope {
        repo_id: Some(repo_id),
        branch: None,
        scope_nonce: 8,
        current_doc: Some(doc_id),
        scope_switch_pending: false,
    };
    assert!(
        evaluate_recovery(
            &required(repo_id, doc_id, ProjectionRecoveryCause::ExternalApply),
            &scope,
        )
        .is_none()
    );
}

#[test]
fn projection_recovery_cycle_merges_duplicates_and_bounds_trailing_reopen() {
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::new();
    let first = required(repo_id, doc_id, ProjectionRecoveryCause::ExternalApply);
    let second = required(repo_id, doc_id, ProjectionRecoveryCause::Merge);
    let third = required(repo_id, doc_id, ProjectionRecoveryCause::PluginMutation);
    let coordinator = ProjectionRecoveryCoordinator::default();

    assert_eq!(coordinator.begin(first.clone()), RecoveryStart::ReopenNow);
    coordinator.mark_generation(11);
    assert_eq!(coordinator.begin(first), RecoveryStart::TrailingQueued);
    assert_eq!(coordinator.begin(second), RecoveryStart::TrailingQueued);
    assert_eq!(
        coordinator.begin(third.clone()),
        RecoveryStart::TrailingQueued
    );
    assert_eq!(
        coordinator.finish_generation(11),
        RecoveryCompletion::ReopenTrailing(third)
    );
    coordinator.mark_generation(12);
    assert_eq!(
        coordinator.finish_generation(12),
        RecoveryCompletion::Finished
    );
    assert!(!coordinator.is_active());
}

#[test]
fn disconnect_reset_allows_next_invalidation_to_reopen_immediately() {
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::new();
    let required = required(repo_id, doc_id, ProjectionRecoveryCause::ExternalApply);
    let coordinator = ProjectionRecoveryCoordinator::default();

    assert_eq!(
        coordinator.begin(required.clone()),
        RecoveryStart::ReopenNow
    );
    coordinator.mark_generation(11);
    coordinator.reset();
    assert_eq!(coordinator.begin(required), RecoveryStart::ReopenNow);
}

#[test]
fn projection_recovery_refresh_is_scope_local_and_bounds_trailing_group() {
    let repo_id = RepoId::new_v4();
    let first_doc = DocId::new();
    let second_doc = DocId::new();
    let first = required(repo_id, first_doc, ProjectionRecoveryCause::ExternalApply);
    let second = required(repo_id, second_doc, ProjectionRecoveryCause::Merge);
    let coordinator = ProjectionRefreshCoordinator::default();
    let scope = ProjectionRefreshScope {
        connection_epoch: 4,
        repo_id: Some(repo_id),
        branch: None,
        scope_nonce: 7,
        scope_switch_pending: false,
    };
    coordinator.enter_scope(scope.clone());

    let first_work = coordinator
        .begin(first.clone(), first.plan.clone())
        .expect("first refresh work");
    assert!(
        coordinator
            .register_requests(
                first_work.flight_id,
                Some("docs-1".into()),
                Some("changes-1".into())
            )
            .is_none()
    );
    assert!(
        coordinator
            .begin(first.clone(), first.plan.clone())
            .is_none(),
        "an identical invalidation must queue one trailing refresh"
    );
    assert!(
        coordinator
            .begin(second.clone(), second.plan.clone())
            .is_none(),
        "a distinct invalidation must become one trailing refresh"
    );
    assert!(
        coordinator
            .complete_response(ProjectionRefreshResponse::DocList, "docs-1")
            .is_none()
    );
    let trailing = coordinator
        .complete_response(ProjectionRefreshResponse::SourceControl, "changes-1")
        .expect("the trailing group starts after both active responses");
    assert_eq!(trailing.required, second);
    assert!(trailing.plan.refresh_doc_list);
    assert!(trailing.plan.refresh_source_control);
    assert!(trailing.plan.refresh_external_changes);

    coordinator.enter_scope(ProjectionRefreshScope {
        connection_epoch: 4,
        repo_id: Some(repo_id),
        branch: None,
        scope_nonce: 8,
        scope_switch_pending: false,
    });
    assert!(
        coordinator
            .begin(first.clone(), first.plan.clone())
            .is_some(),
        "changing repo scope must retire the old in-flight barrier"
    );
}

#[test]
fn projection_refresh_timeout_retires_active_and_trailing_work() {
    let repo_id = RepoId::new_v4();
    let doc_id = DocId::new();
    let required = required(repo_id, doc_id, ProjectionRecoveryCause::ExternalApply);
    let coordinator = ProjectionRefreshCoordinator::default();
    coordinator.enter_scope(ProjectionRefreshScope {
        connection_epoch: 9,
        repo_id: Some(repo_id),
        branch: None,
        scope_nonce: 7,
        scope_switch_pending: false,
    });
    let work = coordinator
        .begin(required.clone(), required.plan.clone())
        .expect("first work");
    coordinator.register_requests(work.flight_id, Some("docs".into()), None);
    coordinator.begin(required.clone(), required.plan.clone());

    assert!(coordinator.retire(work.flight_id));
    assert!(!coordinator.is_active());
    assert!(!coordinator.retire(work.flight_id));
}
