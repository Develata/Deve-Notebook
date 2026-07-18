//! plan_ref:
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 03_storage/projection#remote-import-projection-writeback

use super::Fixture;
use crate::remote_import::store::RemoteImportStore;
use crate::remote_import::types::{
    RemoteImportApplyRequest, RemoteImportDigest, RemoteImportFailure, RemoteImportFailureKind,
    RemoteImportFailurePhase, RemoteImportProjectionOutcome,
};
use std::io::Cursor;

#[test]
fn pending_projection_receipt_is_never_pruned() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut request = fixture.request();
    request.baseline.ledger_head = 0.into();
    let mut capture = fixture.runtime.begin_prepare(request)?;
    capture.capture_file("pending.md", Cursor::new(b"pending"))?;
    let ready = capture.finish()?;
    let candidate = ready.candidate.as_ref().expect("candidate");
    let apply = RemoteImportApplyRequest {
        request_id: uuid::Uuid::new_v4(),
        session_id: ready.session_id,
        revision: candidate.revision,
        locator_digest: candidate.locator_digest,
        ignore_digest: candidate.ignore_digest,
    };
    let prepared =
        fixture
            .runtime
            .prepare_apply(&fixture.repo, fixture.repo.local_repo_name(), apply)?;
    let pending =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), prepared)?;
    assert_eq!(
        pending.projection_outcome,
        RemoteImportProjectionOutcome::Pending
    );
    fixture.runtime.finish_cleanup_for_test(ready.session_id)?;

    let store = RemoteImportStore::open(fixture.db.clone(), fixture.repo_id)?;
    for index in 0..65u64 {
        let record = store.reserve(
            RemoteImportDigest::of(format!("source-{index}").as_bytes()),
            RemoteImportDigest::of(b"locator"),
            index.into(),
            RemoteImportDigest::of(b"ignore"),
        )?;
        store.fail(
            &record,
            RemoteImportFailure {
                phase: RemoteImportFailurePhase::Capture,
                kind: RemoteImportFailureKind::SourceRead,
            },
        )?;
        store.begin_discard(record.session_id, record.generation, None)?;
        store.finish_cleanup(record.session_id)?;
    }

    let records = store.list_sessions()?;
    assert_eq!(
        records.len(),
        crate::remote_import::store::retention::TERMINAL_RETENTION + 1
    );
    let preserved = records
        .iter()
        .find(|record| record.session_id == ready.session_id)
        .expect("pending Applied session retained");
    assert!(!preserved.cleanup_pending);
    assert_eq!(
        preserved
            .apply_receipt
            .as_ref()
            .expect("receipt")
            .projection_outcome,
        RemoteImportProjectionOutcome::Pending
    );
    Ok(())
}
