//! plan_ref:
//!   - 03_storage/authority#sealed-ledger-change-batch
//!   - 06_backup#remote-import-session-contract

use super::*;
use crate::ledger::schema::REMOTE_IMPORT_RUNTIME;
use crate::remote_import::artifact::{CANDIDATES_DIR, candidate_file};
use crate::remote_import::store::{RUNTIME_KEY, decode_runtime, encode};
use redb::ReadableTable;

fn assert_snapshot_drift(
    mutate: impl FnOnce(&mut RemoteImportApplyRequest),
    expected: RemoteImportBlocker,
) -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = ready_session(&fixture, "notes/imported.md", b"alpha")?;
    let mut request = apply_request(&ready);
    mutate(&mut request);
    let prepared =
        fixture
            .runtime
            .prepare_apply(&fixture.repo, fixture.repo.local_repo_name(), request)?;

    assert!(matches!(
        fixture.runtime.commit_apply(
            &fixture.repo,
            fixture.repo.local_repo_name(),
            prepared
        ),
        Err(RemoteImportError::Stale { blockers, .. }) if blockers == vec![expected]
    ));
    assert_eq!(current_head(&fixture)?, 0);
    assert_eq!(
        fixture.runtime.session(ready.session_id)?.state,
        RemoteImportState::Stale
    );
    Ok(())
}

#[test]
fn locator_and_ignore_admission_drift_persist_stale_without_facts() -> anyhow::Result<()> {
    assert_snapshot_drift(
        |request| request.locator_digest = RemoteImportDigest::of(b"changed locator"),
        RemoteImportBlocker::LocatorBindingDrift,
    )?;
    assert_snapshot_drift(
        |request| request.ignore_digest = RemoteImportDigest::of(b"changed ignore"),
        RemoteImportBlocker::IgnoreSnapshotDrift,
    )
}

#[test]
fn active_session_pointer_drift_rejects_without_facts() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = ready_session(&fixture, "notes/imported.md", b"alpha")?;
    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        apply_request(&ready),
    )?;
    let write = fixture.db.begin_write()?;
    {
        let mut runtime_table = write.open_table(REMOTE_IMPORT_RUNTIME)?;
        let guard = runtime_table.get(&RUNTIME_KEY)?.expect("runtime row");
        let mut runtime = decode_runtime(guard.value())?;
        drop(guard);
        runtime.active_session = None;
        let bytes = encode(&runtime)?;
        runtime_table.insert(&RUNTIME_KEY, bytes.as_slice())?;
    }
    write.commit()?;

    assert!(matches!(
        fixture.runtime.commit_apply(
            &fixture.repo,
            fixture.repo.local_repo_name(),
            prepared
        ),
        Err(RemoteImportError::StaleGeneration(id)) if id == ready.session_id
    ));
    assert_eq!(current_head(&fixture)?, 0);
    assert_eq!(
        fixture.runtime.session(ready.session_id)?.state,
        RemoteImportState::Ready
    );
    Ok(())
}

#[test]
fn candidate_tamper_fails_session_before_authority_write() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = ready_session(&fixture, "notes/imported.md", b"alpha")?;
    let revision = ready.candidate.as_ref().expect("candidate").revision;
    std::fs::write(
        fixture
            .artifact_session(ready.session_id)
            .join(CANDIDATES_DIR)
            .join(candidate_file(revision.get())),
        b"{}",
    )?;

    assert!(matches!(
        fixture.runtime.prepare_apply(
            &fixture.repo,
            fixture.repo.local_repo_name(),
            apply_request(&ready)
        ),
        Err(RemoteImportError::ArtifactTampered(_))
    ));
    assert_eq!(current_head(&fixture)?, 0);
    assert_eq!(
        fixture.runtime.session(ready.session_id)?.state,
        RemoteImportState::Failed
    );
    Ok(())
}
