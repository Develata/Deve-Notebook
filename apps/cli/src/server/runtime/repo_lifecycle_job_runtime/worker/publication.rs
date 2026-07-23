//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Durable terminalization, retry-bounded publication, and retention.

use super::{LifecycleReceipt, ReceiptStore, panic_message};
use crate::server::runtime::repo_lifecycle_job_runtime::{
    RepoLifecycleJobCompletion, RepoLifecycleJobError, RepoLifecycleJobExecutor,
    RepoLifecyclePublicationSink,
};
use futures::FutureExt;
use std::panic::AssertUnwindSafe;
use std::time::Duration;
use uuid::Uuid;

const PUBLICATION_ATTEMPTS: usize = 3;
#[cfg(not(test))]
const PUBLICATION_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(1);
#[cfg(test)]
const PUBLICATION_ATTEMPT_TIMEOUT: Duration = Duration::from_millis(50);

pub(super) async fn complete_and_publish(
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

pub(super) async fn retry_pending_publications(
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
    let receipt = store
        .receipt(request_id)
        .ok_or(RepoLifecycleJobError::NotFound)?;
    let job_id = receipt.job_id;
    let publication = receipt
        .publication
        .clone()
        .ok_or(RepoLifecycleJobError::NotFound)?;
    let mut last_error = None;
    for _ in 0..PUBLICATION_ATTEMPTS {
        let publication = publication.clone();
        let attempt =
            AssertUnwindSafe(async { sink.publish(request_id, job_id, publication).await })
                .catch_unwind();
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

pub(super) fn prune_terminal(
    store: &mut ReceiptStore,
    executor: &dyn RepoLifecycleJobExecutor,
) -> Result<(), RepoLifecycleJobError> {
    store.prune_terminal(|repo_id| {
        std::panic::catch_unwind(AssertUnwindSafe(|| executor.retain_create_receipt(repo_id)))
            .unwrap_or(true)
    })?;
    Ok(())
}
