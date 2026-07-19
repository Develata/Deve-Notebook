//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 07_network#repo-control-wire-contract

use super::*;
use std::collections::HashSet;

#[test]
fn terminal_retention_is_deterministic_bounded_and_protects_control_debt() {
    let now = chrono::Utc::now().timestamp_millis();
    let mut receipts = Vec::new();
    for offset in 0..1026_i64 {
        let request_id = Uuid::new_v4();
        let target = RepoId::new_v4();
        let mut receipt = store::LifecycleReceipt::admitted(
            request_id,
            Uuid::new_v4(),
            target,
            RepoLifecycleJobIntent::remove(target),
        )
        .expect("receipt");
        receipt.complete(RepoLifecycleJobCompletion::not_committed("not committed"));
        receipt.updated_at_ms = now - offset;
        receipts.push(receipt);
    }

    let protected_repo = RepoId::new_v4();
    let projection = std::env::current_dir().expect("absolute cwd");
    let mut protected = store::LifecycleReceipt::admitted(
        Uuid::new_v4(),
        Uuid::new_v4(),
        protected_repo,
        RepoLifecycleJobIntent::create("protected", projection).expect("create intent"),
    )
    .expect("protected receipt");
    protected.complete(RepoLifecycleJobCompletion::succeeded(
        RepoLifecycleSettledPublication::Created {
            repo_id: protected_repo,
            mounted: true,
        },
    ));
    protected.mark_publication_delivered();
    protected.updated_at_ms = now - (25 * 60 * 60 * 1_000);
    let protected_request = protected.request_id;
    receipts.push(protected);

    let pending_repo = RepoId::new_v4();
    let mut pending = store::LifecycleReceipt::admitted(
        Uuid::new_v4(),
        Uuid::new_v4(),
        pending_repo,
        RepoLifecycleJobIntent::remove(pending_repo),
    )
    .expect("pending receipt");
    pending.complete(RepoLifecycleJobCompletion::succeeded(
        RepoLifecycleSettledPublication::Removed {
            repo_id: pending_repo,
            fallback_repo_id: None,
        },
    ));
    pending.updated_at_ms = now - (25 * 60 * 60 * 1_000);
    let pending_request = pending.request_id;
    receipts.push(pending);

    let protected_repos = HashSet::from([protected_repo]);
    let removals = store::retention_removals_for_test(&receipts, now, &protected_repos);
    assert_eq!(removals.len(), 2);
    assert!(!removals.contains(&protected_request));
    assert!(!removals.contains(&pending_request));
}

#[test]
fn receipt_diagnostics_are_bounded_and_publication_error_is_orthogonal() {
    let repo_id = RepoId::new_v4();
    let mut receipt = store::LifecycleReceipt::admitted(
        Uuid::new_v4(),
        Uuid::new_v4(),
        repo_id,
        RepoLifecycleJobIntent::remove(repo_id),
    )
    .expect("receipt");
    let long = "诊".repeat(4096);
    let mut completion = RepoLifecycleJobCompletion::repair_required(long.clone());
    for _ in 0..32 {
        completion = completion.with_cleanup(long.clone());
    }
    receipt.complete(completion);
    assert!(
        receipt
            .primary
            .as_deref()
            .is_some_and(|value| value.len() <= 2048)
    );
    assert_eq!(receipt.cleanup.len(), 8);
    assert!(receipt.cleanup.iter().all(|value| value.len() <= 1024));
    assert_eq!(receipt.publication_attempts, 0);
    assert!(receipt.publication_last_error.is_none());
    assert!(serde_json::to_vec(&receipt).expect("receipt json").len() < 64 * 1024);

    let publication_repo = RepoId::new_v4();
    let mut publication_receipt = store::LifecycleReceipt::admitted(
        Uuid::new_v4(),
        Uuid::new_v4(),
        publication_repo,
        RepoLifecycleJobIntent::remove(publication_repo),
    )
    .expect("publication receipt");
    publication_receipt.complete(RepoLifecycleJobCompletion::succeeded(
        RepoLifecycleSettledPublication::Removed {
            repo_id: publication_repo,
            fallback_repo_id: None,
        },
    ));
    publication_receipt.append_publication_failure(long);
    assert!(publication_receipt.cleanup.is_empty());
    assert_eq!(publication_receipt.publication_attempts, 1);
    assert!(
        publication_receipt
            .publication_last_error
            .as_deref()
            .is_some_and(|value| value.len() <= 1024)
    );
    assert!(
        serde_json::to_vec(&publication_receipt)
            .expect("publication receipt json")
            .len()
            < 64 * 1024
    );
}
