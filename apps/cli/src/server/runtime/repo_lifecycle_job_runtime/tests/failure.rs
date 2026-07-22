//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 07_network#repo-control-wire-contract

use super::*;
use std::future::pending;

struct PanicExecutor;

impl super::super::removal::RepoRemovalPlanner for PanicExecutor {}

impl RepoLifecycleJobExecutor for PanicExecutor {
    fn execute(
        &self,
        _job: AdmittedRepoLifecycleJob,
    ) -> model::JobFuture<RepoLifecycleJobCompletion> {
        panic!("injected executor construction panic")
    }

    fn recover(
        &self,
        _job: AdmittedRepoLifecycleJob,
    ) -> model::JobFuture<RepoLifecycleJobCompletion> {
        panic!("injected recovery construction panic")
    }
}

struct HangingSink;

impl RepoLifecyclePublicationSink for HangingSink {
    fn publish(
        &self,
        _request_id: Uuid,
        _publication: RepoLifecycleSettledPublication,
    ) -> model::JobFuture<Result<(), String>> {
        Box::pin(pending())
    }
}

struct PanicSink;

impl RepoLifecyclePublicationSink for PanicSink {
    fn publish(
        &self,
        _request_id: Uuid,
        _publication: RepoLifecycleSettledPublication,
    ) -> model::JobFuture<Result<(), String>> {
        panic!("injected publication construction panic")
    }
}

#[tokio::test]
async fn cancelled_admission_observer_does_not_cancel_owned_job() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
    let executor = TestExecutor::blocked();
    let runtime =
        RepoLifecycleJobRuntime::start(dir.path(), executor.clone(), Arc::new(TestSink::default()))
            .expect("job runtime");
    let request_id = Uuid::new_v4();
    let (reply, response) = tokio::sync::oneshot::channel();
    runtime
        .commands
        .send(worker::Command::Submit {
            request_id,
            intent: create_intent(&projection, "Cancelled waiter"),
            reply,
        })
        .await
        .expect("submit command");
    drop(response);

    timeout(Duration::from_secs(2), async {
        while executor.executions.load(Ordering::Acquire) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("owned execution start");
    executor.release(1);
    assert_eq!(
        terminal_status(&runtime, request_id).await.outcome,
        Some(RepoLifecycleJobOutcome::Succeeded)
    );
    runtime.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn executor_construction_panic_terminalizes_only_its_job() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
    let runtime = RepoLifecycleJobRuntime::start(
        dir.path(),
        Arc::new(PanicExecutor),
        Arc::new(TestSink::default()),
    )
    .expect("job runtime");
    let request_id = Uuid::new_v4();
    runtime
        .submit(request_id, create_intent(&projection, "Panic"))
        .await
        .expect("admission");

    let status = terminal_status(&runtime, request_id).await;
    assert_eq!(
        status.outcome,
        Some(RepoLifecycleJobOutcome::RepairRequired)
    );
    runtime.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn hanging_and_panicking_publication_remain_bounded_durable_debt() {
    for sink in [
        Arc::new(HangingSink) as Arc<dyn RepoLifecyclePublicationSink>,
        Arc::new(PanicSink) as Arc<dyn RepoLifecyclePublicationSink>,
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
        let executor = TestExecutor::blocked();
        executor.release(1);
        let runtime =
            RepoLifecycleJobRuntime::start(dir.path(), executor, sink).expect("job runtime");
        let request_id = Uuid::new_v4();
        runtime
            .submit(
                request_id,
                create_intent(&projection, "Publication failure"),
            )
            .await
            .expect("admission");

        let status = timeout(
            Duration::from_secs(2),
            terminal_status(&runtime, request_id),
        )
        .await
        .expect("bounded publication attempt");
        assert_eq!(status.outcome, Some(RepoLifecycleJobOutcome::Succeeded));
        assert!(status.publication_pending);
        runtime.shutdown().await.expect("shutdown");
    }
}

#[tokio::test]
async fn post_replace_failure_stops_owner_and_recovery_keeps_original_target() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
    let runtime = RepoLifecycleJobRuntime::start(
        dir.path(),
        TestExecutor::blocked(),
        Arc::new(TestSink::default()),
    )
    .expect("job runtime");
    let receipt_dir = dir.path().join(".host").join("repo-lifecycle-jobs");
    let marker = receipt_dir.join(store::POST_REPLACE_FAILURE_MARKER);
    std::fs::write(&marker, "inject").expect("failure marker");
    let request_id = Uuid::new_v4();
    let error = runtime
        .submit(request_id, create_intent(&projection, "Post replace"))
        .await
        .expect_err("post-replace failure must fail admission");
    assert!(matches!(error, RepoLifecycleJobError::Store(_)));
    let receipt_path = receipt_dir.join(format!("{request_id}.json"));
    let value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt_path).expect("durable receipt"))
            .expect("receipt json");
    let original_target = Uuid::parse_str(
        value["target_repo_id"]
            .as_str()
            .expect("target RepoId string"),
    )
    .expect("target RepoId");
    runtime.abort_for_test().await;
    std::fs::remove_file(marker).expect("remove failure marker");

    let recovery = TestExecutor::blocked();
    recovery.release(1);
    let reopened =
        RepoLifecycleJobRuntime::start(dir.path(), recovery, Arc::new(TestSink::default()))
            .expect("recovery runtime");
    let status = terminal_status(&reopened, request_id).await;
    assert_eq!(status.target_repo_id, original_target);
    assert_eq!(
        status.outcome,
        Some(RepoLifecycleJobOutcome::RepairRequired)
    );
    reopened.shutdown().await.expect("shutdown");
}
