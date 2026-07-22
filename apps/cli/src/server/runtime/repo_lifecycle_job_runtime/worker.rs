//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator

use super::model::{
    AdmittedRepoLifecycleJob, RepoLifecycleJobAccepted, RepoLifecycleJobCompletion,
    RepoLifecycleJobError, RepoLifecycleJobExecutor, RepoLifecycleJobIntent,
    RepoLifecycleJobStatus, RepoLifecyclePublicationSink,
};
use super::removal::{RemovalProgressCommand, RepoRemovalExecution, RepoRemovalProgress};
use super::removal::{RepoRemovalExecuteIntent, RepoRemovalPrepareIntent, RepoRemovalPrepared};
use super::store::RemovalPreparationState;
use super::store::{LifecycleReceipt, ReceiptStore};
use deve_core::models::RepoId;
use futures::FutureExt;
use std::collections::{HashSet, VecDeque};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use uuid::Uuid;

const MAX_ACTIVE_JOBS: usize = 4;
const PUBLICATION_ATTEMPTS: usize = 3;
#[cfg(not(test))]
const PUBLICATION_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(1);
#[cfg(test)]
const PUBLICATION_ATTEMPT_TIMEOUT: Duration = Duration::from_millis(50);

mod progress;
mod removal;

pub(super) enum Command {
    Submit {
        request_id: Uuid,
        intent: RepoLifecycleJobIntent,
        reply: oneshot::Sender<Result<RepoLifecycleJobAccepted, RepoLifecycleJobError>>,
    },
    Status {
        request_id: Uuid,
        reply: oneshot::Sender<Result<RepoLifecycleJobStatus, RepoLifecycleJobError>>,
    },
    PrepareRemoval {
        intent: RepoRemovalPrepareIntent,
        reply: oneshot::Sender<Result<RepoRemovalPrepared, RepoLifecycleJobError>>,
    },
    ExecuteRemoval {
        intent: RepoRemovalExecuteIntent,
        now_ms: Option<i64>,
        reply: oneshot::Sender<Result<RepoLifecycleJobAccepted, RepoLifecycleJobError>>,
    },
    Shutdown {
        reply: oneshot::Sender<Result<(), RepoLifecycleJobError>>,
    },
}

struct FinishedJob {
    request_id: Uuid,
    target_repo_id: RepoId,
    result: Result<RepoLifecycleJobCompletion, String>,
}

pub(super) async fn run(
    mut store: ReceiptStore,
    executor: Arc<dyn RepoLifecycleJobExecutor>,
    publication_sink: Arc<dyn RepoLifecyclePublicationSink>,
    runtime_incarnation: Uuid,
    mut commands: mpsc::Receiver<Command>,
) -> Result<(), RepoLifecycleJobError> {
    let (progress_tx, mut progress_rx) = mpsc::channel(32);
    prune_terminal(&mut store, executor.as_ref())?;
    let mut jobs = JoinSet::new();
    let mut active_repos = HashSet::new();
    let mut recovery = VecDeque::from(store.active_receipts());
    spawn_recovery_jobs(
        &mut store,
        &executor,
        &mut jobs,
        &mut active_repos,
        &mut recovery,
        &progress_tx,
    )?;
    retry_pending_publications(&mut store, publication_sink.as_ref()).await?;
    prune_terminal(&mut store, executor.as_ref())?;

    let mut shutdown_reply = None;
    loop {
        if shutdown_reply.is_some() && jobs.is_empty() && recovery.is_empty() {
            break;
        }
        tokio::select! {
            command = commands.recv(), if shutdown_reply.is_none() => {
                match command {
                    Some(Command::Submit { request_id, intent, reply }) => {
                        let result = admit(
                            &mut store,
                            &executor,
                            &mut jobs,
                            &mut active_repos,
                            request_id,
                            intent,
                        );
                        match result {
                            Err(RepoLifecycleJobError::Store(detail)) => {
                                let _ = reply.send(Err(RepoLifecycleJobError::Store(detail.clone())));
                                return Err(RepoLifecycleJobError::Store(detail));
                            }
                            result => {
                                let _ = reply.send(result);
                            }
                        }
                    }
                    Some(Command::Status { request_id, reply }) => {
                        let result = store
                            .receipt(request_id)
                            .map(LifecycleReceipt::status)
                            .ok_or(RepoLifecycleJobError::NotFound);
                        let _ = reply.send(result);
                    }
                    Some(Command::PrepareRemoval { intent, reply }) => {
                        let result = removal::prepare_removal(
                            &mut store,
                            executor.as_ref(),
                            runtime_incarnation,
                            intent,
                        ).await;
                        match result {
                            Err(RepoLifecycleJobError::Store(detail)) => {
                                let _ = reply.send(Err(RepoLifecycleJobError::Store(detail.clone())));
                                return Err(RepoLifecycleJobError::Store(detail));
                            }
                            result => { let _ = reply.send(result); }
                        }
                    }
                    Some(Command::ExecuteRemoval { intent, now_ms, reply }) => {
                        let result = removal::execute_removal(
                            removal::ExecuteRemovalContext {
                                store: &mut store,
                                executor: &executor,
                                jobs: &mut jobs,
                                active_repos: &mut active_repos,
                                runtime_incarnation,
                                progress_tx: progress_tx.clone(),
                            },
                            now_ms.unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
                            intent,
                        ).await;
                        match result {
                            Err(RepoLifecycleJobError::Store(detail)) => {
                                let _ = reply.send(Err(RepoLifecycleJobError::Store(detail.clone())));
                                return Err(RepoLifecycleJobError::Store(detail));
                            }
                            result => { let _ = reply.send(result); }
                        }
                    }
                    Some(Command::Shutdown { reply }) => {
                        commands.close();
                        reject_queued_commands(&mut commands);
                        shutdown_reply = Some(reply);
                    }
                    None => break,
                }
            }
            progress = progress_rx.recv(), if !jobs.is_empty() => {
                let Some(progress) = progress else {
                    return Err(RepoLifecycleJobError::Coordination("removal progress channel stopped"));
                };
                let result = progress::apply(&mut store, progress);
                if let Err(RepoLifecycleJobError::Store(detail)) = &result {
                    return Err(RepoLifecycleJobError::Store(detail.clone()));
                }
            }
            finished = jobs.join_next(), if !jobs.is_empty() => {
                let Some(finished) = finished else { continue };
                let finished = finished.map_err(|_| {
                    RepoLifecycleJobError::Coordination("lifecycle owner task failed")
                })?;
                active_repos.remove(&finished.target_repo_id);
                let completion = match finished.result {
                    Ok(completion) => completion,
                    Err(error) => RepoLifecycleJobCompletion::repair_required(error),
                };
                complete_and_publish(
                    &mut store,
                    publication_sink.as_ref(),
                    executor.as_ref(),
                    finished.request_id,
                    completion,
                ).await?;
                spawn_recovery_jobs(
                    &mut store,
                    &executor,
                    &mut jobs,
                    &mut active_repos,
                    &mut recovery,
                    &progress_tx,
                )?;
            }
        }
    }
    retry_pending_publications(&mut store, publication_sink.as_ref()).await?;
    prune_terminal(&mut store, executor.as_ref())?;
    if let Some(reply) = shutdown_reply {
        let _ = reply.send(Ok(()));
    }
    Ok(())
}

fn admit(
    store: &mut ReceiptStore,
    executor: &Arc<dyn RepoLifecycleJobExecutor>,
    jobs: &mut JoinSet<FinishedJob>,
    active_repos: &mut HashSet<RepoId>,
    request_id: Uuid,
    intent: RepoLifecycleJobIntent,
) -> Result<RepoLifecycleJobAccepted, RepoLifecycleJobError> {
    if request_id.is_nil() {
        return Err(RepoLifecycleJobError::InvalidRequest);
    }
    intent.validate()?;
    if intent.operation() == super::model::RepoLifecycleJobOperation::Remove {
        return Err(RepoLifecycleJobError::InvalidRequest);
    }
    if let Some(receipt) = store.receipt(request_id) {
        if !receipt.matches_intent(&intent)? {
            return Err(RepoLifecycleJobError::RequestConflict);
        }
        return Ok(RepoLifecycleJobAccepted {
            request_id,
            job_id: receipt.job_id,
            target_repo_id: receipt.target_repo_id,
        });
    }
    if store.request_id_is_bound(request_id) {
        return Err(RepoLifecycleJobError::RequestConflict);
    }
    if jobs.len() >= MAX_ACTIVE_JOBS {
        return Err(RepoLifecycleJobError::Busy);
    }
    let target_repo_id = intent.requested_repo_id().unwrap_or_else(RepoId::new_v4);
    if active_repos.contains(&target_repo_id) {
        return Err(RepoLifecycleJobError::Busy);
    }
    let job_id = Uuid::new_v4();
    let receipt = LifecycleReceipt::admitted(request_id, job_id, target_repo_id, intent.clone())?;
    store.insert(receipt)?;
    active_repos.insert(target_repo_id);
    let receipt = store
        .receipt(request_id)
        .expect("inserted lifecycle receipt must remain addressable");
    spawn_job(jobs, executor.clone(), admitted(receipt), None, false);
    Ok(RepoLifecycleJobAccepted {
        request_id,
        job_id,
        target_repo_id,
    })
}

fn spawn_recovery_jobs(
    store: &mut ReceiptStore,
    executor: &Arc<dyn RepoLifecycleJobExecutor>,
    jobs: &mut JoinSet<FinishedJob>,
    active_repos: &mut HashSet<RepoId>,
    recovery: &mut VecDeque<LifecycleReceipt>,
    progress_tx: &mpsc::Sender<RemovalProgressCommand>,
) -> Result<(), RepoLifecycleJobError> {
    while jobs.len() < MAX_ACTIVE_JOBS {
        let Some(receipt) = recovery.pop_front() else {
            break;
        };
        if !active_repos.insert(receipt.target_repo_id) {
            return Err(RepoLifecycleJobError::Coordination(
                "multiple interrupted lifecycle jobs target one RepoId",
            ));
        }
        let receipt = store.update(receipt.request_id, LifecycleReceipt::mark_recovering)?;
        let removal = if receipt.operation == super::model::RepoLifecycleJobOperation::Remove {
            Some(removal_execution_for_request(
                store,
                receipt.request_id,
                progress_tx.clone(),
            )?)
        } else {
            None
        };
        spawn_job(jobs, executor.clone(), admitted(&receipt), removal, true);
    }
    Ok(())
}

fn spawn_job(
    jobs: &mut JoinSet<FinishedJob>,
    executor: Arc<dyn RepoLifecycleJobExecutor>,
    job: AdmittedRepoLifecycleJob,
    removal: Option<RepoRemovalExecution>,
    recovery: bool,
) {
    jobs.spawn(async move {
        let request_id = job.request_id;
        let target_repo_id = job.target_repo_id;
        let result = AssertUnwindSafe(async move {
            let future = match (recovery, removal) {
                (true, Some(removal)) => executor.recover_removal(job, removal),
                (false, Some(removal)) => executor.execute_removal(job, removal),
                (true, None) => executor.recover(job),
                (false, None) => executor.execute(job),
            };
            future.await
        })
        .catch_unwind()
        .await
        .map_err(panic_message);
        FinishedJob {
            request_id,
            target_repo_id,
            result,
        }
    });
}

pub(super) fn removal_execution_for_request(
    store: &ReceiptStore,
    execute_request_id: Uuid,
    sender: mpsc::Sender<RemovalProgressCommand>,
) -> Result<RepoRemovalExecution, RepoLifecycleJobError> {
    let record = store
        .removal_by_execute_request(execute_request_id)
        .ok_or(RepoLifecycleJobError::NotFound)?;
    let manifest = record
        .manifest
        .clone()
        .ok_or(RepoLifecycleJobError::RemovalBlocked)?;
    let manifest_digest = record
        .manifest_digest
        .clone()
        .ok_or(RepoLifecycleJobError::RemovalBlocked)?;
    let state = match &record.state {
        RemovalPreparationState::ExecuteAdmitted { execution, .. } => execution.as_ref().clone(),
        _ => return Err(RepoLifecycleJobError::ConfirmationInvalid),
    };
    Ok(RepoRemovalExecution {
        preparation_id: record.preparation_id,
        execute_request_id,
        manifest_digest,
        manifest,
        state,
        progress: RepoRemovalProgress::new(record.preparation_id, execute_request_id, sender),
    })
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<&str>() {
        return format!("lifecycle executor panicked: {message}");
    }
    if let Some(message) = panic.downcast_ref::<String>() {
        return format!("lifecycle executor panicked: {message}");
    }
    "lifecycle executor panicked with a non-string payload".to_string()
}

fn admitted(receipt: &LifecycleReceipt) -> AdmittedRepoLifecycleJob {
    AdmittedRepoLifecycleJob {
        request_id: receipt.request_id,
        job_id: receipt.job_id,
        target_repo_id: receipt.target_repo_id,
        intent: receipt.intent.clone(),
    }
}

async fn complete_and_publish(
    store: &mut ReceiptStore,
    sink: &dyn RepoLifecyclePublicationSink,
    executor: &dyn RepoLifecycleJobExecutor,
    request_id: Uuid,
    completion: RepoLifecycleJobCompletion,
) -> Result<(), RepoLifecycleJobError> {
    if store.removal_has_committed_debt_for_request(request_id) {
        return Ok(());
    }
    let receipt = match store.receipt(request_id) {
        Some(receipt) if receipt.phase.is_terminal() => receipt.clone(),
        _ => store.update(request_id, |receipt| receipt.complete(completion))?,
    };
    if receipt.publication_pending {
        publish_one(store, sink, request_id).await?;
    }
    prune_terminal(store, executor)?;
    Ok(())
}

async fn retry_pending_publications(
    store: &mut ReceiptStore,
    sink: &dyn RepoLifecyclePublicationSink,
) -> Result<(), RepoLifecycleJobError> {
    for request_id in store.pending_publications() {
        publish_one(store, sink, request_id).await?;
    }
    Ok(())
}

async fn publish_one(
    store: &mut ReceiptStore,
    sink: &dyn RepoLifecyclePublicationSink,
    request_id: Uuid,
) -> Result<(), RepoLifecycleJobError> {
    let publication = store
        .receipt(request_id)
        .and_then(|receipt| receipt.publication.clone())
        .ok_or(RepoLifecycleJobError::NotFound)?;
    let mut last_error = None;
    for _ in 0..PUBLICATION_ATTEMPTS {
        let publication = publication.clone();
        let attempt =
            AssertUnwindSafe(async { sink.publish(request_id, publication).await }).catch_unwind();
        match tokio::time::timeout(PUBLICATION_ATTEMPT_TIMEOUT, attempt).await {
            Ok(Ok(Ok(()))) => {
                store.update(request_id, LifecycleReceipt::mark_publication_delivered)?;
                return Ok(());
            }
            Ok(Ok(Err(error))) => last_error = Some(error),
            Ok(Err(panic)) => last_error = Some(panic_message(panic)),
            Err(_) => last_error = Some("publication attempt timed out".to_string()),
        }
    }
    if let Some(error) = last_error {
        store.update(request_id, |receipt| {
            receipt.append_publication_failure(error)
        })?;
    }
    Ok(())
}

fn prune_terminal(
    store: &mut ReceiptStore,
    executor: &dyn RepoLifecycleJobExecutor,
) -> Result<(), RepoLifecycleJobError> {
    store.prune_terminal(|repo_id| {
        std::panic::catch_unwind(AssertUnwindSafe(|| executor.retain_create_receipt(repo_id)))
            .unwrap_or(true)
    })?;
    Ok(())
}

fn reject_queued_commands(commands: &mut mpsc::Receiver<Command>) {
    while let Ok(command) = commands.try_recv() {
        match command {
            Command::Submit { reply, .. } => {
                let _ = reply.send(Err(RepoLifecycleJobError::AdmissionClosed));
            }
            Command::Status {
                request_id: _,
                reply,
            } => {
                let _ = reply.send(Err(RepoLifecycleJobError::AdmissionClosed));
            }
            Command::PrepareRemoval { reply, .. } => {
                let _ = reply.send(Err(RepoLifecycleJobError::AdmissionClosed));
            }
            Command::ExecuteRemoval { reply, .. } => {
                let _ = reply.send(Err(RepoLifecycleJobError::AdmissionClosed));
            }
            Command::Shutdown { reply } => {
                let _ = reply.send(Ok(()));
            }
        }
    }
}
