//! plan_ref:
//!   - 06_backup#remote-import-state-machine
//!   - 03_storage/repair#remote-import-cleanup-repair

use super::*;
use crate::remote_import::artifact::{CANDIDATES_DIR, candidate_file};
use redb::ReadableTable;

#[test]
fn repair_distinguishes_missing_candidate_and_orphan_temp() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;
    let candidates = fixture.artifact_session(session_id).join(CANDIDATES_DIR);
    std::fs::remove_file(candidates.join(candidate_file(1)))?;
    std::fs::write(candidates.join(".2.preparing-test"), b"partial")?;

    let report = fixture.repair()?;
    assert!(
        report
            .findings
            .contains(&RemoteImportRepairFinding::MissingCandidate {
                session_id,
                revision: 1,
            })
    );
    assert!(report.findings.iter().any(|finding| {
        matches!(finding, RemoteImportRepairFinding::OrphanCandidateTemp { session_id: id, .. } if *id == session_id)
    }));
    Ok(())
}

#[test]
fn repair_reports_unreferenced_blob_and_unknown_session_entry() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;
    let session = fixture.artifact_session(session_id);
    let extra_digest = RemoteImportDigest::of(b"orphan").to_hex();
    std::fs::write(session.join("blobs").join(&extra_digest), b"orphan")?;
    std::fs::write(session.join("unexpected.txt"), b"unexpected")?;

    let report = fixture.repair()?;

    assert!(
        report
            .findings
            .contains(&RemoteImportRepairFinding::ExtraBlob {
                session_id,
                digest: extra_digest,
            })
    );
    assert!(
        report
            .findings
            .contains(&RemoteImportRepairFinding::UnknownSessionArtifact {
                session_id,
                name: "unexpected.txt".into(),
            })
    );
    Ok(())
}

#[test]
fn repair_accepts_applied_record_after_artifact_cleanup() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    let mut applied = capture.finish()?;
    let candidate = applied.candidate.as_ref().expect("candidate").clone();
    applied.state = RemoteImportState::Applied;
    applied.apply_receipt = Some(crate::remote_import::types::RemoteImportApplyReceipt {
        request_id: uuid::Uuid::new_v4(),
        session_id,
        revision: candidate.revision,
        writer_peer_id: crate::models::PeerId::new("writer"),
        authority_head_before: candidate.ledger_head,
        authority_head_after: candidate
            .ledger_head
            .next()
            .expect("test authority head fits"),
        manifest_digest: applied
            .source_snapshot
            .as_ref()
            .expect("source snapshot")
            .manifest_digest,
        candidate_digest: candidate.candidate_digest,
        projection_outcome: crate::remote_import::types::RemoteImportProjectionOutcome::Written,
    });
    applied.cleanup_pending = false;
    let bytes = crate::codec::encode(&applied)?;
    let write = fixture.db.begin_write()?;
    {
        let mut sessions = write.open_table(REMOTE_IMPORT_SESSIONS)?;
        sessions.insert(&session_id.as_u128(), bytes.as_slice())?;
        let mut runtime_table = write.open_table(REMOTE_IMPORT_RUNTIME)?;
        let runtime_guard = runtime_table.get(&0)?.expect("runtime row");
        let mut runtime: crate::remote_import::types::RemoteImportRuntimeRecord =
            crate::codec::decode(runtime_guard.value())?;
        drop(runtime_guard);
        runtime.active_session = None;
        let runtime_bytes = crate::codec::encode(&runtime)?;
        runtime_table.insert(&0, runtime_bytes.as_slice())?;
    }
    write.commit()?;
    std::fs::remove_dir_all(fixture.artifact_session(session_id))?;

    let report = fixture.repair()?;

    assert!(!report.findings.iter().any(|finding| {
        matches!(
            finding,
            RemoteImportRepairFinding::MissingArtifact(id) if *id == session_id
        )
    }));
    Ok(())
}

#[cfg(unix)]
#[test]
fn discard_rejects_dangling_session_symlink_and_keeps_cleanup_pending() -> anyhow::Result<()> {
    use std::os::unix::fs::symlink;

    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;
    let session_path = fixture.artifact_session(session_id);
    std::fs::remove_dir_all(&session_path)?;
    symlink(session_path.join("missing-target"), &session_path)?;

    let error = fixture
        .runtime
        .discard(session_id)
        .expect_err("dangling symlink must not be treated as absent");

    assert!(matches!(error, RemoteImportError::UnsafeArtifactRoot(_)));
    let record = fixture.runtime.session(session_id)?;
    assert_eq!(record.state, RemoteImportState::Discarded);
    assert!(record.cleanup_pending);
    assert!(
        std::fs::symlink_metadata(session_path)?
            .file_type()
            .is_symlink()
    );
    Ok(())
}

#[test]
fn publication_tamper_cannot_reach_ready() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();

    let error = capture
        .finish_with_before_ready_test(|session| {
            let blob = std::fs::read_dir(session.join("blobs"))?
                .next()
                .expect("blob")?
                .path();
            std::fs::write(blob, b"tampered")?;
            Ok(())
        })
        .expect_err("tampered publication must not become Ready");

    assert!(matches!(error, RemoteImportError::ArtifactTampered(_)));
    let record = fixture.runtime.session(session_id)?;
    assert_eq!(record.state, RemoteImportState::Failed);
    assert!(record.source_snapshot.is_none());
    assert!(record.candidate.is_none());
    Ok(())
}

#[test]
fn discard_preserves_cleanup_debt_when_ready_blob_is_tampered_same_length() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;
    let blob = std::fs::read_dir(fixture.artifact_session(session_id).join("blobs"))?
        .next()
        .expect("blob")?
        .path();
    std::fs::write(&blob, b"gamma")?;

    let error = fixture
        .runtime
        .discard(session_id)
        .expect_err("tampered Ready artifacts must remain repair-visible");

    assert!(matches!(error, RemoteImportError::ArtifactTampered(_)));
    let record = fixture.runtime.session(session_id)?;
    assert_eq!(record.state, RemoteImportState::Discarded);
    assert!(record.cleanup_pending);
    assert!(blob.is_file());
    Ok(())
}
