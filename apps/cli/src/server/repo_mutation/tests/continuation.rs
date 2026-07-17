//! plan_ref:
//!   - 03_storage/authority#repo-mutation-publication-gate
//!
//! Cancellation, continuation, and failure-cut tests for mounted repository lanes.

use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn aborted_caller_cannot_release_blocking_repo_side_effect_permit() {
    let repo_id = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(repo_id, 1, RepoMountState::Mounted);
    let gate = Arc::new(RepoMutationPublicationGate::new(view));
    let admission = gate
        .admit_mounted_repo(repo_id)
        .expect("mounted repo admission");
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();

    let blocking_gate = gate.clone();
    let blocking = tokio::spawn(async move {
        blocking_gate
            .execute_admitted_mounted_repo_unpublished_blocking_with_continuation(
                admission,
                move || {
                    entered_tx.send(()).expect("announce blocking side effect");
                    release_rx.recv().expect("release blocking side effect");
                    Ok::<_, ()>(())
                },
            )
            .await
    });
    entered_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("blocking side effect entered");
    blocking.abort();
    assert!(matches!(blocking.await, Err(error) if error.is_cancelled()));

    let second_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = second_calls.clone();
    let second_gate = gate.clone();
    let (tx, _) = broadcast::channel(2);
    let second = tokio::spawn(async move {
        second_gate
            .execute_mounted_repo(repo_id, &tx, || {
                observed_calls.fetch_add(1, Ordering::SeqCst);
                MutationExecution::<(), ()>::not_committed(())
            })
            .await
    });
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert_eq!(
        second_calls.load(Ordering::SeqCst),
        0,
        "the blocking task, not its cancelled caller, owns the repo permit"
    );

    release_tx.send(()).expect("release blocking side effect");
    tokio::time::timeout(Duration::from_secs(2), second)
        .await
        .expect("second mutation must resume after blocking side effect")
        .expect("second mutation task")
        .expect("second mutation execution");
    assert_eq!(second_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocking_repo_side_effect_restores_nested_lane_guard() {
    let repo_id = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(repo_id, 1, RepoMountState::Mounted);
    let gate = Arc::new(RepoMutationPublicationGate::new(view));
    let admission = gate
        .admit_mounted_repo(repo_id)
        .expect("mounted repo admission");
    let nested_gate = gate.clone();
    let (tx, _) = broadcast::channel(2);

    let (outcome, _continuation) = gate
        .execute_admitted_mounted_repo_unpublished_blocking_with_continuation(
            admission,
            move || {
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
            },
        )
        .await
        .expect("blocking execution");
    outcome.expect("blocking operation");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admitted_continuation_can_finalize_after_repo_local_failure() {
    let repo_id = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(repo_id, 9, RepoMountState::Mounted);
    let gate = RepoMutationPublicationGate::new(view.clone());
    let admission = gate
        .admit_mounted_repo(repo_id)
        .expect("mounted repo admission");
    let (applied, continuation) = gate
        .execute_admitted_mounted_repo_unpublished_blocking_with_continuation(admission, || {
            Ok::<_, ()>("applied")
        })
        .await
        .expect("initial apply");
    assert_eq!(applied.expect("apply result"), "applied");

    view.set_state_for_test(repo_id, RepoMountState::Failed);
    let finalized = gate
        .execute_mounted_repo_continuation_unpublished_blocking(continuation, || {
            Ok::<_, ()>("finalized")
        })
        .await
        .expect("failed watcher does not invalidate admitted cleanup");
    assert_eq!(finalized.expect("finalizer"), "finalized");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admitted_continuation_cannot_cross_lifecycle_transition() {
    let repo_id = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(repo_id, 9, RepoMountState::Mounted);
    let gate = RepoMutationPublicationGate::new(view.clone());
    let admission = gate
        .admit_mounted_repo(repo_id)
        .expect("mounted repo admission");
    let (_applied, continuation) = gate
        .execute_admitted_mounted_repo_unpublished_blocking_with_continuation(admission, || {
            Ok::<_, ()>(())
        })
        .await
        .expect("initial apply");

    view.set_state_for_test(repo_id, RepoMountState::Transitioning);
    let error = gate
        .execute_mounted_repo_continuation_unpublished_blocking(continuation, || Ok::<_, ()>(()))
        .await
        .expect_err("lifecycle transition must invalidate continuation");
    assert_eq!(
        error.server_error().code,
        ServerErrorCode::StorageWorkspaceIngestionUnavailable
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn admitted_continuation_cannot_cross_intervening_repo_writer() {
    let repo_id = RepoId::new_v4();
    let runtime = WatcherRuntimeView::with_state_for_test(repo_id, 1, RepoMountState::Mounted);
    let gate = RepoMutationPublicationGate::new(runtime);
    let admission = gate.admit_mounted_repo(repo_id).expect("admission");
    let (_outcome, continuation) = gate
        .execute_admitted_mounted_repo_unpublished_blocking_with_continuation(admission, || {
            Ok::<_, anyhow::Error>(())
        })
        .await
        .expect("initial apply");

    gate.execute_mounted_repo_unpublished(repo_id, || Ok::<_, anyhow::Error>(()))
        .await
        .expect("intervening gate")
        .expect("intervening writer");
    let finalized = Arc::new(AtomicBool::new(false));
    let observed = finalized.clone();
    let error = gate
        .execute_mounted_repo_continuation_unpublished_blocking(continuation, move || {
            observed.store(true, Ordering::SeqCst);
            Ok::<_, anyhow::Error>(())
        })
        .await
        .expect_err("intervening writer must supersede continuation");

    assert_eq!(error, RepoMutationGateError::ContinuationSuperseded);
    assert!(!finalized.load(Ordering::SeqCst));
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
