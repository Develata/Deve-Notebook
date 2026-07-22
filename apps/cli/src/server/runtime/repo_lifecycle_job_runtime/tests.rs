//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!   - 07_network#repo-control-wire-contract

use super::model::{
    AdmittedRepoLifecycleJob, RepoLifecycleJobCompletion, RepoLifecycleJobOperation,
    RepoLifecycleJobOutcome, RepoLifecycleJobPhase, RepoLifecycleSettledPublication,
};
use super::*;
use deve_core::models::RepoId;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::Semaphore;
use tokio::time::{Duration, timeout};

mod failure;
mod removal;
mod removal_store;
mod retention;

struct TestExecutor {
    permits: Arc<Semaphore>,
    executions: AtomicUsize,
    recoveries: AtomicUsize,
}

impl super::removal::RepoRemovalPlanner for TestExecutor {}

impl TestExecutor {
    fn blocked() -> Arc<Self> {
        Arc::new(Self {
            permits: Arc::new(Semaphore::new(0)),
            executions: AtomicUsize::new(0),
            recoveries: AtomicUsize::new(0),
        })
    }

    fn release(&self, count: usize) {
        self.permits.add_permits(count);
    }

    fn completion(job: AdmittedRepoLifecycleJob) -> RepoLifecycleJobCompletion {
        match job.intent.operation() {
            RepoLifecycleJobOperation::Create => {
                RepoLifecycleJobCompletion::succeeded(RepoLifecycleSettledPublication::Created {
                    repo_id: job.target_repo_id,
                    mounted: true,
                })
            }
            RepoLifecycleJobOperation::Remove => {
                RepoLifecycleJobCompletion::succeeded(RepoLifecycleSettledPublication::Removed {
                    repo_id: job.target_repo_id,
                    fallback_repo_id: None,
                })
            }
        }
    }
}

impl RepoLifecycleJobExecutor for TestExecutor {
    fn execute(
        &self,
        job: AdmittedRepoLifecycleJob,
    ) -> model::JobFuture<RepoLifecycleJobCompletion> {
        self.executions.fetch_add(1, Ordering::AcqRel);
        let permits = self.permits.clone();
        Box::pin(async move {
            let permit = permits.acquire().await.expect("test executor semaphore");
            permit.forget();
            Self::completion(job)
        })
    }

    fn recover(
        &self,
        job: AdmittedRepoLifecycleJob,
    ) -> model::JobFuture<RepoLifecycleJobCompletion> {
        self.recoveries.fetch_add(1, Ordering::AcqRel);
        let permits = self.permits.clone();
        Box::pin(async move {
            let permit = permits.acquire().await.expect("test recovery semaphore");
            permit.forget();
            RepoLifecycleJobCompletion::repair_required(format!(
                "interrupted lifecycle job {} converged through test truth",
                job.request_id
            ))
        })
    }
}

#[derive(Default)]
struct TestSink {
    failures_remaining: AtomicUsize,
    publications: std::sync::Mutex<Vec<(Uuid, RepoLifecycleSettledPublication)>>,
}

impl TestSink {
    fn failing(attempts: usize) -> Arc<Self> {
        Arc::new(Self {
            failures_remaining: AtomicUsize::new(attempts),
            publications: std::sync::Mutex::new(Vec::new()),
        })
    }

    fn count(&self) -> usize {
        self.publications.lock().expect("test publications").len()
    }
}

impl RepoLifecyclePublicationSink for TestSink {
    fn publish(
        &self,
        request_id: Uuid,
        publication: RepoLifecycleSettledPublication,
    ) -> model::JobFuture<Result<(), String>> {
        let should_fail = self
            .failures_remaining
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok();
        if !should_fail {
            self.publications
                .lock()
                .expect("test publications")
                .push((request_id, publication));
        }
        Box::pin(async move {
            if should_fail {
                Err("injected publication failure".to_string())
            } else {
                Ok(())
            }
        })
    }
}

fn create_intent(root: &std::path::Path, alias: &str) -> RepoLifecycleJobIntent {
    RepoLifecycleJobIntent::create(alias, root, None).expect("create intent")
}

async fn terminal_status(
    runtime: &RepoLifecycleJobRuntime,
    request_id: Uuid,
) -> RepoLifecycleJobStatus {
    timeout(Duration::from_secs(2), async {
        loop {
            let status = runtime.status(request_id).await.expect("job status");
            if status.phase == RepoLifecycleJobPhase::Terminal {
                return status;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("terminal lifecycle status")
}

#[tokio::test]
async fn same_request_is_idempotent_and_conflicting_parameters_fail_closed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
    let executor = TestExecutor::blocked();
    let sink = TestSink::default();
    let runtime = RepoLifecycleJobRuntime::start(dir.path(), executor.clone(), Arc::new(sink))
        .expect("job runtime");
    let request_id = Uuid::new_v4();
    let intent = create_intent(&projection, "Research");

    let first = runtime
        .submit(request_id, intent.clone())
        .await
        .expect("first admission");
    let retry = runtime
        .submit(request_id, intent)
        .await
        .expect("idempotent retry");
    assert_eq!(first, retry);
    assert_eq!(executor.executions.load(Ordering::Acquire), 1);

    let conflict = runtime
        .submit(request_id, create_intent(&projection, "Different"))
        .await
        .expect_err("request parameters must remain immutable");
    assert!(matches!(conflict, RepoLifecycleJobError::RequestConflict));

    executor.release(1);
    let status = terminal_status(&runtime, request_id).await;
    assert_eq!(status.outcome, Some(RepoLifecycleJobOutcome::Succeeded));
    runtime.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn dropping_transport_observation_does_not_cancel_accepted_job() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
    let executor = TestExecutor::blocked();
    let sink = Arc::new(TestSink::default());
    let runtime = RepoLifecycleJobRuntime::start(dir.path(), executor.clone(), sink.clone())
        .expect("job runtime");
    let request_id = Uuid::new_v4();

    let observer = runtime
        .submit(request_id, create_intent(&projection, "Detached observer"))
        .await
        .expect("admitted");
    let _ = observer;
    executor.release(1);

    let status = terminal_status(&runtime, request_id).await;
    assert_eq!(status.outcome, Some(RepoLifecycleJobOutcome::Succeeded));
    assert!(!status.publication_pending);
    assert_eq!(sink.count(), 1);
    runtime.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn global_active_job_bound_fails_busy_without_receipt() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
    let executor = TestExecutor::blocked();
    let runtime =
        RepoLifecycleJobRuntime::start(dir.path(), executor.clone(), Arc::new(TestSink::default()))
            .expect("job runtime");
    // Removal no longer enters this generic submit path in F4/v5; its exact
    // RepoId single-flight is owned by Prepare/Execute admission tests.
    for index in 0..4 {
        runtime
            .submit(
                Uuid::new_v4(),
                create_intent(&projection, &format!("repo-{index}")),
            )
            .await
            .expect("bounded admission");
    }
    let overflow_id = Uuid::new_v4();
    let overflow = runtime
        .submit(overflow_id, create_intent(&projection, "overflow"))
        .await
        .expect_err("fifth active job must fail busy");
    assert!(matches!(overflow, RepoLifecycleJobError::Busy));
    assert!(matches!(
        runtime.status(overflow_id).await,
        Err(RepoLifecycleJobError::NotFound)
    ));

    executor.release(4);
    runtime.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn interrupted_receipt_is_recovered_without_allocating_new_identity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
    let first_executor = TestExecutor::blocked();
    let first_runtime =
        RepoLifecycleJobRuntime::start(dir.path(), first_executor, Arc::new(TestSink::default()))
            .expect("first runtime");
    let request_id = Uuid::new_v4();
    let intent = create_intent(&projection, "Recovered");
    let admitted = first_runtime
        .submit(request_id, intent.clone())
        .await
        .expect("admission");
    first_runtime.abort_for_test().await;

    let recovery_executor = TestExecutor::blocked();
    recovery_executor.release(1);
    let second_runtime = RepoLifecycleJobRuntime::start(
        dir.path(),
        recovery_executor.clone(),
        Arc::new(TestSink::default()),
    )
    .expect("recovery runtime");
    let status = terminal_status(&second_runtime, request_id).await;
    assert_eq!(status.job_id, admitted.job_id);
    assert_eq!(status.target_repo_id, admitted.target_repo_id);
    assert_eq!(
        status.outcome,
        Some(RepoLifecycleJobOutcome::RepairRequired)
    );
    assert_eq!(recovery_executor.recoveries.load(Ordering::Acquire), 1);
    let retry = second_runtime
        .submit(request_id, intent)
        .await
        .expect("recovered request retry");
    assert_eq!(retry, admitted);
    second_runtime.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn publication_debt_is_durable_and_replayed_by_next_runtime() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
    let executor = TestExecutor::blocked();
    executor.release(1);
    let failing_sink = TestSink::failing(usize::MAX);
    let runtime =
        RepoLifecycleJobRuntime::start(dir.path(), executor, failing_sink).expect("job runtime");
    let request_id = Uuid::new_v4();
    runtime
        .submit(request_id, create_intent(&projection, "Publication debt"))
        .await
        .expect("admission");
    let status = terminal_status(&runtime, request_id).await;
    assert!(status.publication_pending);
    runtime.shutdown().await.expect("shutdown");

    let recovery = TestExecutor::blocked();
    let sink = Arc::new(TestSink::default());
    let reopened = RepoLifecycleJobRuntime::start(dir.path(), recovery, sink.clone())
        .expect("reopened runtime");
    timeout(Duration::from_secs(2), async {
        loop {
            if !reopened
                .status(request_id)
                .await
                .expect("status")
                .publication_pending
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("publication replay");
    assert_eq!(sink.count(), 1);
    reopened.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn receipt_identity_tamper_fails_closed_on_reopen() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
    let executor = TestExecutor::blocked();
    let runtime =
        RepoLifecycleJobRuntime::start(dir.path(), executor, Arc::new(TestSink::default()))
            .expect("job runtime");
    let request_id = Uuid::new_v4();
    runtime
        .submit(request_id, create_intent(&projection, "Tamper"))
        .await
        .expect("admission");
    runtime.abort_for_test().await;

    let receipt = dir
        .path()
        .join(".host")
        .join("repo-lifecycle-jobs")
        .join(format!("{request_id}.json"));
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&receipt).expect("read receipt"))
            .expect("receipt json");
    value["request_id"] = serde_json::Value::String(Uuid::new_v4().to_string());
    std::fs::write(
        &receipt,
        serde_json::to_vec_pretty(&value).expect("tampered json"),
    )
    .expect("tamper receipt");

    let reopened = RepoLifecycleJobRuntime::start(
        dir.path(),
        TestExecutor::blocked(),
        Arc::new(TestSink::default()),
    );
    let reopened = match reopened {
        Ok(_) => panic!("tampered receipt must fail closed"),
        Err(error) => error,
    };
    assert!(matches!(reopened, RepoLifecycleJobError::Store(_)));
}

#[tokio::test]
async fn explicit_shutdown_closes_admission_and_joins_owned_jobs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projection = std::fs::canonicalize(dir.path()).expect("canonical projection");
    let executor = TestExecutor::blocked();
    let runtime =
        RepoLifecycleJobRuntime::start(dir.path(), executor.clone(), Arc::new(TestSink::default()))
            .expect("job runtime");
    let request_id = Uuid::new_v4();
    runtime
        .submit(request_id, create_intent(&projection, "Shutdown"))
        .await
        .expect("admission");
    let shutdown_runtime = runtime.clone();
    let shutdown = tokio::spawn(async move { shutdown_runtime.shutdown().await });
    tokio::task::yield_now().await;
    let rejected = runtime
        .submit(Uuid::new_v4(), create_intent(&projection, "Late"))
        .await
        .expect_err("shutdown must close admission");
    assert!(matches!(rejected, RepoLifecycleJobError::AdmissionClosed));
    assert!(!shutdown.is_finished());
    shutdown.abort();
    let resumed_runtime = runtime.clone();
    let resumed_shutdown = tokio::spawn(async move { resumed_runtime.shutdown().await });
    executor.release(1);
    resumed_shutdown
        .await
        .expect("shutdown join")
        .expect("shutdown");
}
