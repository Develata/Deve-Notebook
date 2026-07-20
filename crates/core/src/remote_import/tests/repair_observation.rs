//! plan_ref:
//!   - 03_storage/authority#remote-import-workflow-tables
//!   - 03_storage/repair#remote-import-cleanup-repair

use super::{Fixture, artifact_tree_bytes, durable_bytes};
use crate::remote_import::artifact::{ArtifactCapture, RemoteImportArtifactRoot};
use crate::remote_import::repair::RemoteImportRepairFinding;
use crate::remote_import::store::RemoteImportStore;
use std::io::Cursor;

#[test]
fn dry_run_repair_is_observational_only() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;

    let durable_before = durable_bytes(&fixture.db, session_id)?;
    let artifacts_before = artifact_tree_bytes(&fixture.artifact_session(session_id))?;
    let first = fixture.repair()?;
    let second = fixture.repair()?;

    assert_eq!(first, second);
    assert_eq!(durable_before, durable_bytes(&fixture.db, session_id)?);
    assert_eq!(
        artifacts_before,
        artifact_tree_bytes(&fixture.artifact_session(session_id))?
    );
    Ok(())
}

#[test]
fn repair_reports_orphan_staging_without_cleanup() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let capture = fixture.runtime.begin_prepare(fixture.request())?;
    let session_id = capture.session_id();
    drop(capture);

    let report = fixture.repair()?;
    assert!(report.findings.iter().any(|finding| {
        matches!(
            finding,
            RemoteImportRepairFinding::OrphanPreparingArtifact(name)
                if name.starts_with(&format!(".{session_id}.preparing-"))
        )
    }));
    Ok(())
}

#[test]
fn repair_reports_final_artifact_published_before_ready_cas() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let store = RemoteImportStore::open(fixture.db.clone(), fixture.repo_id)?;
    let request = fixture.request();
    let record = store.reserve(
        request.source_binding_digest,
        request.locator_binding_digest,
        request.baseline.ledger_head,
        request.baseline.ignore_digest,
    )?;
    let root = RemoteImportArtifactRoot::open(&fixture.ledger, fixture.repo_id)?;
    let mut capture = ArtifactCapture::start(root, record.session_id, record.generation)?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    capture.finish(&request.baseline)?;

    let report = fixture.repair()?;
    assert!(
        report
            .findings
            .contains(&RemoteImportRepairFinding::IncompletePublication(
                record.session_id
            ))
    );
    Ok(())
}
