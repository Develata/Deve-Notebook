//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 06_backup#remote-import-runtime-boundary
//!
//! Deterministic dry-run repair-plan projection.

use super::super::error::RemoteImportResult;
use super::super::repair::{RemoteImportRepairFinding, RemoteImportRepairReport};
use super::types::RemoteImportRepairPlan;
use sha2::{Digest, Sha256};

pub(super) fn repair_plan(
    report: RemoteImportRepairReport,
) -> RemoteImportResult<RemoteImportRepairPlan> {
    let repairable_count = report
        .findings
        .iter()
        .filter(|finding| {
            matches!(
                finding,
                RemoteImportRepairFinding::CleanupPending(_)
                    | RemoteImportRepairFinding::OrphanSessionArtifact(_)
            )
        })
        .count();
    let mut hasher = Sha256::new();
    hasher.update(b"deve-remote-import-repair-plan-v1\0");
    for finding in &report.findings {
        let encoded = format!("{finding:?}");
        hasher.update((encoded.len() as u64).to_le_bytes());
        hasher.update(encoded.as_bytes());
    }
    Ok(RemoteImportRepairPlan {
        finding_count: report.findings.len(),
        repairable_count,
        token: hex::encode(hasher.finalize()),
    })
}
