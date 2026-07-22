//! plan_ref:
//!   - 04_repository#repo-lifecycle-coordinator
//!
//! Deterministic bounded lifecycle receipt retention.

use super::{
    LifecycleReceipt, RepoLifecycleJobOperation, TERMINAL_RECEIPT_LIMIT, TERMINAL_RETENTION_MS,
};
use deve_core::models::RepoId;
use uuid::Uuid;

pub(super) fn terminal_retention_removals<'a>(
    rows: impl Iterator<Item = &'a LifecycleReceipt>,
    now_ms: i64,
    retain_normal_create: &mut impl FnMut(RepoId) -> bool,
) -> Vec<Uuid> {
    let cutoff = now_ms.saturating_sub(TERMINAL_RETENTION_MS);
    let mut candidates = rows
        .filter(|receipt| {
            receipt.phase.is_terminal()
                && !receipt.publication_pending
                && !(receipt.operation == RepoLifecycleJobOperation::Create
                    && retain_normal_create(receipt.target_repo_id))
        })
        .map(|receipt| (receipt.request_id, receipt.updated_at_ms))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, (request_id, updated_at_ms))| {
            (index >= TERMINAL_RECEIPT_LIMIT || updated_at_ms < cutoff).then_some(request_id)
        })
        .collect()
}
