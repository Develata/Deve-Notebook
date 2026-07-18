//! plan_ref:
//!   - 03_storage/projection#durable-projection-fault-contract
//!   - 03_storage/projection#remote-import-projection-writeback

use super::{apply_request, ready_session};
use crate::ledger::schema::PROJECTION_FAULTS;
use crate::remote_import::{RemoteImportDigest, RemoteImportError, RemoteImportProjectionOutcome};
use redb::ReadableTable;

#[test]
fn projection_written_settlement_is_monotonic_and_idempotent() -> anyhow::Result<()> {
    let fixture = super::super::Fixture::new()?;
    let ready = ready_session(&fixture, "notes/written.md", b"alpha")?;
    let request = apply_request(&ready);
    let prepared =
        fixture
            .runtime
            .prepare_apply(&fixture.repo, fixture.repo.local_repo_name(), request)?;
    let pending =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), prepared)?;

    let written = fixture.runtime.settle_projection_written(&pending)?;
    assert_eq!(
        written.projection_outcome,
        RemoteImportProjectionOutcome::Written
    );
    assert_eq!(
        fixture.runtime.settle_projection_written(&pending)?,
        written
    );
    assert_eq!(projection_fault_count(&fixture)?, 0);
    assert!(matches!(
        fixture
            .runtime
            .settle_projection_degraded(&pending, "late failure"),
        Err(RemoteImportError::ApplyFailed(message))
            if message.contains("terminal outcome")
    ));
    assert_eq!(projection_fault_count(&fixture)?, 0);
    Ok(())
}

#[test]
fn projection_degraded_settlement_atomically_binds_typed_fault() -> anyhow::Result<()> {
    let fixture = super::super::Fixture::new()?;
    let ready = ready_session(&fixture, "notes/degraded.md", b"alpha")?;
    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        apply_request(&ready),
    )?;
    let pending =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), prepared)?;

    let degraded = fixture
        .runtime
        .settle_projection_degraded(&pending, "workspace writeback failed")?;
    assert_eq!(
        degraded.projection_outcome,
        RemoteImportProjectionOutcome::Degraded
    );
    assert_eq!(projection_fault_count(&fixture)?, 1);
    assert_eq!(
        crate::projection_fault::remote_import_origins_for_test(
            fixture.db.as_ref(),
            fixture.repo_id
        )?,
        vec![(
            pending.session_id.as_u128(),
            pending.revision.get(),
            pending.request_id,
        )]
    );
    assert_eq!(
        fixture
            .runtime
            .settle_projection_degraded(&pending, "replayed detail")?,
        degraded
    );
    assert_eq!(projection_fault_count(&fixture)?, 1);
    assert!(matches!(
        fixture.runtime.settle_projection_written(&pending),
        Err(RemoteImportError::ApplyFailed(message))
            if message.contains("terminal outcome")
    ));
    Ok(())
}

#[test]
fn aborted_degraded_settlement_leaves_pending_without_fault() -> anyhow::Result<()> {
    let fixture = super::super::Fixture::new()?;
    let ready = ready_session(&fixture, "notes/rollback.md", b"alpha")?;
    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        apply_request(&ready),
    )?;
    let pending =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), prepared)?;
    let info = fixture.repo.get_repo_info()?.expect("repo info");
    let fault = crate::projection_fault::prepare_remote_import_fault(
        fixture.repo_id,
        info.name,
        pending.session_id.as_u128(),
        pending.revision.get(),
        pending.request_id,
        pending.authority_head_after.storage_key(),
        "injected transaction abort",
    );

    let uncommitted = crate::remote_import::apply::settle_degraded_without_commit_for_test(
        &crate::remote_import::store::RemoteImportStore::open(fixture.db.clone(), fixture.repo_id)?,
        &pending,
        &fault,
    )?;
    assert_eq!(
        uncommitted.projection_outcome,
        RemoteImportProjectionOutcome::Degraded
    );
    assert_eq!(projection_fault_count(&fixture)?, 0);
    assert_eq!(
        fixture
            .runtime
            .session(ready.session_id)?
            .apply_receipt
            .expect("receipt")
            .projection_outcome,
        RemoteImportProjectionOutcome::Pending
    );
    Ok(())
}

#[test]
fn stale_projection_settlement_receipt_writes_no_fault() -> anyhow::Result<()> {
    let fixture = super::super::Fixture::new()?;
    let ready = ready_session(&fixture, "notes/stale-settlement.md", b"alpha")?;
    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        apply_request(&ready),
    )?;
    let mut stale =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), prepared)?;
    stale.candidate_digest = RemoteImportDigest::of(b"wrong candidate");

    assert!(matches!(
        fixture
            .runtime
            .settle_projection_degraded(&stale, "must not persist"),
        Err(RemoteImportError::ArtifactTampered(_))
    ));
    assert_eq!(projection_fault_count(&fixture)?, 0);
    Ok(())
}

fn projection_fault_count(fixture: &super::super::Fixture) -> anyhow::Result<usize> {
    let read = fixture.db.begin_read()?;
    let table = read.open_table(PROJECTION_FAULTS)?;
    Ok(table.iter()?.count())
}
