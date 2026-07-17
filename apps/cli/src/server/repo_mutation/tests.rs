mod continuation;

use super::{
    MutationExecution, MutationPublication, RepoMutationGateError, RepoMutationPublicationGate,
};
use crate::server::runtime::watcher_runtime::{RepoMountState, WatcherRuntimeView};
use deve_core::models::RepoId;
use deve_core::protocol::{
    DocumentRecoveryScope, ProjectionRecoveryCause, ProjectionRecoveryPlan,
    ProjectionRecoveryRequired, ServerErrorCode, ServerMessage,
};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::time::Duration;
use tokio::sync::{Notify, broadcast};

fn gate() -> RepoMutationPublicationGate {
    RepoMutationPublicationGate::new(WatcherRuntimeView::permissive_for_tests())
}

fn publication(repo_id: RepoId) -> MutationPublication {
    MutationPublication::ProjectionRecovery(ProjectionRecoveryRequired {
        repo_id,
        branch: None,
        scope_nonce: None,
        cause: ProjectionRecoveryCause::DocumentMutation,
        plan: ProjectionRecoveryPlan {
            documents: DocumentRecoveryScope::None,
            refresh_doc_list: true,
            refresh_source_control: true,
            refresh_external_changes: false,
        },
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serializes_same_repo_and_preserves_publication_order() {
    let gate = Arc::new(gate());
    let repo_id = RepoId::new_v4();
    let (tx, mut rx) = broadcast::channel(8);
    let entered = Arc::new(Notify::new());
    let order = Arc::new(Mutex::new(Vec::new()));

    let first_gate = gate.clone();
    let first_tx = tx.clone();
    let first_entered = entered.clone();
    let first_order = order.clone();
    let first = tokio::spawn(async move {
        first_gate
            .execute_mounted_repo(repo_id, &first_tx, move || {
                first_entered.notify_one();
                std::thread::sleep(Duration::from_millis(75));
                first_order.lock().expect("order lock").push(1);
                MutationExecution::<_, ()>::committed(1, publication(repo_id))
            })
            .await
            .expect("first execution")
    });

    entered.notified().await;
    let second_gate = gate.clone();
    let second_tx = tx.clone();
    let second_order = order.clone();
    let second = tokio::spawn(async move {
        second_gate
            .execute_mounted_repo(repo_id, &second_tx, || {
                second_order.lock().expect("order lock").push(2);
                MutationExecution::<_, ()>::committed(2, publication(repo_id))
            })
            .await
            .expect("second execution")
    });

    assert!(matches!(
        first.await.expect("first task"),
        MutationExecution::Committed { value: 1, .. }
    ));
    assert!(matches!(
        second.await.expect("second task"),
        MutationExecution::Committed { value: 2, .. }
    ));
    assert_eq!(*order.lock().expect("order lock"), [1, 2]);
    for _ in 0..2 {
        assert!(matches!(
            rx.recv().await.expect("publication"),
            ServerMessage::ProjectionRecoveryRequired(_)
        ));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn different_repos_can_execute_in_parallel() {
    let gate = Arc::new(gate());
    let (tx, _) = broadcast::channel(8);
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(2));
    let mut tasks = Vec::new();

    for repo_id in [RepoId::new_v4(), RepoId::new_v4()] {
        let gate = gate.clone();
        let tx = tx.clone();
        let active = active.clone();
        let max_active = max_active.clone();
        let barrier = barrier.clone();
        tasks.push(tokio::spawn(async move {
            gate.execute_mounted_repo(repo_id, &tx, || {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(now, Ordering::SeqCst);
                barrier.wait();
                active.fetch_sub(1, Ordering::SeqCst);
                MutationExecution::<_, ()>::committed((), publication(repo_id))
            })
            .await
            .expect("repo execution")
        }));
    }
    for task in tasks {
        task.await.expect("parallel task");
    }
    assert_eq!(max_active.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn weak_registry_does_not_retain_repo_locks() {
    let gate = gate();
    let (tx, _) = broadcast::channel(2);
    gate.execute_mounted_repo(RepoId::new_v4(), &tx, || {
        MutationExecution::<(), ()>::not_committed(())
    })
    .await
    .expect("repo execution");
    assert_eq!(gate.live_registry_len().expect("registry"), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nested_same_repo_fails_closed() {
    let gate = Arc::new(gate());
    let (tx, _) = broadcast::channel(2);
    let repo_id = RepoId::new_v4();
    let nested_gate = gate.clone();
    let nested_tx = tx.clone();

    gate.execute_mounted_repo(repo_id, &tx, || {
        let nested = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(nested_gate.execute_mounted_repo(
                repo_id,
                &nested_tx,
                || MutationExecution::<(), ()>::not_committed(()),
            ))
        });
        assert!(
            nested
                .expect_err("nested permit must fail")
                .to_string()
                .contains("nested repo mutation permit")
        );
        MutationExecution::<(), ()>::not_committed(())
    })
    .await
    .expect("outer execution");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repo_to_catalog_nesting_fails_closed() {
    let gate = Arc::new(gate());
    let (tx, _) = broadcast::channel(2);
    let repo_id = RepoId::new_v4();
    let nested_gate = gate.clone();
    let nested_tx = tx.clone();

    gate.execute_mounted_repo(repo_id, &tx, || {
        let nested = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(nested_gate.execute_catalog(&nested_tx, || {
                    MutationExecution::<(), ()>::not_committed(())
                }))
        });
        assert!(
            nested
                .expect_err("repo-to-catalog nesting must fail")
                .to_string()
                .contains("nested repository catalog mutation permit")
        );
        MutationExecution::<(), ()>::not_committed(())
    })
    .await
    .expect("outer execution");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn repo_to_other_repo_catalog_nesting_fails_closed() {
    let gate = Arc::new(gate());
    let (tx, _) = broadcast::channel(2);
    let first_repo = RepoId::new_v4();
    let second_repo = RepoId::new_v4();
    let nested_gate = gate.clone();
    let nested_tx = tx.clone();

    gate.execute_mounted_repo(first_repo, &tx, || {
        let nested = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(nested_gate.execute_catalog_repo(
                second_repo,
                &nested_tx,
                || MutationExecution::<(), ()>::not_committed(()),
            ))
        });
        assert!(matches!(
            nested,
            Err(super::RepoMutationGateError::NestedLane(
                super::gate::MutationLane::Repo(repo_id)
            )) if repo_id == second_repo
        ));
        MutationExecution::<(), ()>::not_committed(())
    })
    .await
    .expect("outer execution");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn catalog_repo_lane_excludes_same_repo_writer() {
    let gate = Arc::new(gate());
    let (tx, _) = broadcast::channel(4);
    let repo_id = RepoId::new_v4();
    let entered = Arc::new(Notify::new());
    let order = Arc::new(Mutex::new(Vec::new()));

    let lifecycle_gate = gate.clone();
    let lifecycle_tx = tx.clone();
    let lifecycle_entered = entered.clone();
    let lifecycle_order = order.clone();
    let lifecycle = tokio::spawn(async move {
        lifecycle_gate
            .execute_catalog_repo(repo_id, &lifecycle_tx, || {
                lifecycle_entered.notify_one();
                std::thread::sleep(Duration::from_millis(75));
                lifecycle_order
                    .lock()
                    .expect("order lock")
                    .push("lifecycle");
                MutationExecution::<(), ()>::not_committed(())
            })
            .await
            .expect("lifecycle execution");
    });

    entered.notified().await;
    let writer_gate = gate.clone();
    let writer_tx = tx.clone();
    let writer_order = order.clone();
    let writer = tokio::spawn(async move {
        writer_gate
            .execute_mounted_repo(repo_id, &writer_tx, || {
                writer_order.lock().expect("order lock").push("writer");
                MutationExecution::<(), ()>::not_committed(())
            })
            .await
            .expect("writer execution");
    });

    lifecycle.await.expect("lifecycle task");
    writer.await.expect("writer task");
    assert_eq!(*order.lock().expect("order lock"), ["lifecycle", "writer"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 3)]
async fn mounted_repo_gate_revalidates_after_waiting_for_repo_permit() {
    let repo_id = RepoId::new_v4();
    let view = WatcherRuntimeView::with_state_for_test(repo_id, 1, RepoMountState::Mounted);
    let gate = Arc::new(RepoMutationPublicationGate::new(view.clone()));
    let (tx, _) = broadcast::channel(4);
    let entered = Arc::new(Notify::new());
    let (release_tx, release_rx) = std::sync::mpsc::channel();

    let first_gate = gate.clone();
    let first_tx = tx.clone();
    let first_entered = entered.clone();
    let first = tokio::spawn(async move {
        first_gate
            .execute_mounted_repo(repo_id, &first_tx, move || {
                first_entered.notify_one();
                release_rx.recv().expect("release first mutation");
                MutationExecution::<(), ()>::not_committed(())
            })
            .await
    });
    entered.notified().await;

    let calls = Arc::new(AtomicUsize::new(0));
    let second_calls = calls.clone();
    let second_gate = gate.clone();
    let second_tx = tx.clone();
    let second = tokio::spawn(async move {
        second_gate
            .execute_mounted_repo(repo_id, &second_tx, || {
                second_calls.fetch_add(1, Ordering::SeqCst);
                MutationExecution::<(), ()>::not_committed(())
            })
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    view.set_state_for_test(repo_id, RepoMountState::Failed);
    release_tx.send(()).expect("release first mutation");

    first.await.expect("first task").expect("first execution");
    let error = second
        .await
        .expect("second task")
        .expect_err("failed mount must close queued mutation");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        error.server_error().code,
        ServerErrorCode::StorageWorkspaceIngestionUnavailable
    );
}
