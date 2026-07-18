//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Cancellation, blocking-lane, and failure-cut tests for mounted repository lanes.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn aborted_caller_cannot_release_blocking_repo_side_effect_permit() {
    let repo_id = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(repo_id, 1, RepoMountState::Mounted);
    let gate = Arc::new(RepoMutationPublicationGate::new(view));
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();

    let blocking_gate = gate.clone();
    let blocking = tokio::spawn(async move {
        blocking_gate
            .execute_mounted_repo_unpublished_blocking(repo_id, move || {
                entered_tx.send(()).expect("announce blocking side effect");
                release_rx.recv().expect("release blocking side effect");
                Ok::<_, ()>(())
            })
            .await
    });
    entered_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("blocking side effect entered");
    blocking.abort();
    assert!(matches!(blocking.await, Err(error) if error.is_cancelled()));

    let second_called = Arc::new(AtomicBool::new(false));
    let observed_call = second_called.clone();
    let second_gate = gate.clone();
    let (tx, _) = broadcast::channel(2);
    let second = tokio::spawn(async move {
        second_gate
            .execute_mounted_repo(repo_id, &tx, || {
                observed_call.store(true, Ordering::SeqCst);
                MutationExecution::<(), ()>::not_committed(())
            })
            .await
    });
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(
        !second_called.load(Ordering::SeqCst),
        "the blocking task, not its cancelled caller, owns the repo permit"
    );

    release_tx.send(()).expect("release blocking side effect");
    tokio::time::timeout(Duration::from_secs(2), second)
        .await
        .expect("second mutation must resume after blocking side effect")
        .expect("second mutation task")
        .expect("second mutation execution");
    assert!(second_called.load(Ordering::SeqCst));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_repo_side_effect_restores_nested_lane_guard() {
    let repo_id = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(repo_id, 1, RepoMountState::Mounted);
    let gate = Arc::new(RepoMutationPublicationGate::new(view));
    let nested_gate = gate.clone();
    let (tx, _) = broadcast::channel(2);

    let outcome = gate
        .execute_mounted_repo_unpublished_blocking(repo_id, move || {
            let nested = tokio::runtime::Handle::current().block_on(
                nested_gate.execute_mounted_repo(repo_id, &tx, || {
                    MutationExecution::<(), ()>::not_committed(())
                }),
            );
            assert!(matches!(
                nested,
                Err(RepoMutationGateError::NestedLane(
                    super::super::gate::MutationLane::Repo(observed)
                )) if observed == repo_id
            ));
            Ok::<_, ()>(())
        })
        .await
        .expect("blocking execution");
    outcome.expect("blocking operation");
}

#[tokio::test]
async fn mounted_repo_gate_allows_only_operations_admitted_before_failure_cut() {
    let repo_id = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(repo_id, 1, RepoMountState::Mounted);
    let gate = RepoMutationPublicationGate::new(view.clone());
    let (tx, mut rx) = broadcast::channel(4);

    let execution = gate
        .execute_mounted_repo(repo_id, &tx, || {
            view.set_state_for_test(repo_id, RepoMountState::Failed);
            MutationExecution::<(), ()>::committed((), publication(repo_id))
        })
        .await
        .expect("already admitted operation may complete");
    assert!(matches!(execution, MutationExecution::Committed { .. }));
    assert!(matches!(
        rx.try_recv().expect("publication for admitted operation"),
        ServerMessage::ProjectionRecoveryRequired(_)
    ));

    let calls = AtomicUsize::new(0);
    let error = gate
        .execute_mounted_repo(repo_id, &tx, || {
            calls.fetch_add(1, Ordering::SeqCst);
            MutationExecution::<(), ()>::not_committed(())
        })
        .await
        .expect_err("later operation must observe failed mount");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        error.server_error().detail.as_deref(),
        Some("Workspace changes are temporarily unavailable; restart the service to recover.")
    );
}
