//! plan_ref:
//!   - 03_storage/authority#sealed-ledger-change-batch
//!   - 06_backup#remote-import-session-contract

use super::*;
use crate::ledger::schema::REMOTE_IMPORT_SESSIONS;
use crate::remote_import::store::{decode_session, encode};
use crate::remote_import::{RemoteImportApplyReceipt, RemoteImportSessionId};
use redb::ReadableTable;

fn apply_once(
    fixture: &Fixture,
) -> anyhow::Result<(
    crate::remote_import::RemoteImportSessionRecord,
    RemoteImportApplyRequest,
    RemoteImportApplyReceipt,
)> {
    let ready = ready_session(fixture, "notes/imported.md", b"alpha")?;
    let request = apply_request(&ready);
    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        request.clone(),
    )?;
    let receipt =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), prepared)?;
    Ok((ready, request, receipt))
}

fn mutate_receipt(
    fixture: &Fixture,
    session_id: RemoteImportSessionId,
    mutate: impl FnOnce(&mut RemoteImportApplyReceipt),
) -> anyhow::Result<()> {
    let write = fixture.db.begin_write()?;
    {
        let mut sessions = write.open_table(REMOTE_IMPORT_SESSIONS)?;
        let guard = sessions
            .get(&session_id.as_u128())?
            .expect("applied session");
        let mut record = decode_session(session_id.as_u128(), guard.value(), fixture.repo_id)?;
        drop(guard);
        mutate(record.apply_receipt.as_mut().expect("stored receipt"));
        let bytes = encode(&record)?;
        sessions.insert(&session_id.as_u128(), bytes.as_slice())?;
    }
    write.commit()?;
    Ok(())
}

#[test]
fn exact_replay_survives_cleanup_completion() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let (ready, request, receipt) = apply_once(&fixture)?;
    let committed_head = current_head(&fixture)?;
    let cleaned = fixture.runtime.finish_cleanup_for_test(ready.session_id)?;
    assert!(!cleaned.cleanup_pending);

    let replay =
        fixture
            .runtime
            .prepare_apply(&fixture.repo, fixture.repo.local_repo_name(), request)?;
    let replayed =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), replay)?;
    assert_eq!(replayed, receipt);
    assert_eq!(current_head(&fixture)?, committed_head);
    Ok(())
}

#[test]
fn exact_replay_survives_a_new_active_session() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let (_ready, request, receipt) = apply_once(&fixture)?;
    let committed_head = current_head(&fixture)?;
    let next_capture = fixture.runtime.begin_prepare(fixture.request())?;

    let replay =
        fixture
            .runtime
            .prepare_apply(&fixture.repo, fixture.repo.local_repo_name(), request)?;
    let replayed =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), replay)?;
    assert_eq!(replayed, receipt);
    assert_eq!(current_head(&fixture)?, committed_head);
    next_capture.abort()?;
    Ok(())
}

#[test]
fn replay_returns_latest_projection_outcome_without_append() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let (ready, request, _receipt) = apply_once(&fixture)?;
    let committed_head = current_head(&fixture)?;
    let replay =
        fixture
            .runtime
            .prepare_apply(&fixture.repo, fixture.repo.local_repo_name(), request)?;
    mutate_receipt(&fixture, ready.session_id, |receipt| {
        receipt.projection_outcome = RemoteImportProjectionOutcome::Written;
    })?;

    let replayed =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), replay)?;
    assert_eq!(
        replayed.projection_outcome,
        RemoteImportProjectionOutcome::Written
    );
    assert_eq!(current_head(&fixture)?, committed_head);
    Ok(())
}

#[test]
fn replay_rejects_corrupt_stored_receipt_core() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let (ready, request, _receipt) = apply_once(&fixture)?;
    let committed_head = current_head(&fixture)?;
    mutate_receipt(&fixture, ready.session_id, |receipt| {
        receipt.candidate_digest = RemoteImportDigest::of(b"corrupt candidate core");
    })?;

    assert!(matches!(
        fixture
            .runtime
            .prepare_apply(&fixture.repo, fixture.repo.local_repo_name(), request),
        Err(RemoteImportError::ArtifactTampered(_))
    ));
    assert_eq!(current_head(&fixture)?, committed_head);
    Ok(())
}
