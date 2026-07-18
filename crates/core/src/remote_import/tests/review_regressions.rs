//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 06_backup#remote-import-state-machine
//!   - 03_storage/repair#remote-import-cleanup-repair

use super::*;
use crate::remote_import::artifact::{CANDIDATES_DIR, validate_remote_path};
use crate::remote_import::types::{
    RemoteImportCandidateRevision, RemoteImportFailure, RemoteImportFailurePhase,
};
use redb::ReadableTable;
use std::io::{Error as IoError, Read};

mod refresh_concurrency;
mod repair_cleanup;

#[test]
fn runtime_rejects_repo_id_mismatch_before_artifact_or_durable_mutation() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let wrong_repo_id = uuid::Uuid::new_v4();
    let before = table_snapshot(&fixture.db)?;
    let wrong_root = crate::utils::notegit::host_dir(&fixture.ledger)
        .join("remote-imports")
        .join(wrong_repo_id.to_string());

    let error =
        RemoteImportRuntime::open_for_test(fixture.db.clone(), &fixture.ledger, wrong_repo_id)
            .err()
            .expect("mismatched RepoId must fail closed");

    assert!(
        matches!(error, RemoteImportError::Storage(message) if message.contains("does not match"))
    );
    assert_eq!(table_snapshot(&fixture.db)?, before);
    assert!(!wrong_root.exists());
    Ok(())
}

#[test]
fn corrupt_store_open_is_observational_only() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let capture = fixture.runtime.begin_prepare(fixture.request())?;
    std::mem::forget(capture);
    let write = fixture.db.begin_write()?;
    {
        let mut runtime = write.open_table(REMOTE_IMPORT_RUNTIME)?;
        runtime.remove(&0)?;
    }
    write.commit()?;
    let before = table_snapshot(&fixture.db)?;

    let error = RemoteImportStore::open(fixture.db.clone(), fixture.repo_id)
        .err()
        .expect("missing runtime row with sessions must fail");

    assert!(matches!(error, RemoteImportError::Storage(message) if message.contains("missing")));
    assert_eq!(table_snapshot(&fixture.db)?, before);
    Ok(())
}

#[test]
fn repair_dry_run_does_not_initialize_empty_store_or_artifact_root() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger, 8, Some("notes"), Some("urn:notes"))?;
    let repo_id = repo.get_repo_info()?.expect("RepoInfo").uuid;
    let before = table_snapshot(repo.local_db.as_ref())?;
    let artifact_root = crate::utils::notegit::host_dir(&ledger)
        .join("remote-imports")
        .join(repo_id.to_string());
    assert!(!artifact_root.exists());

    let report = RemoteImportRuntime::dry_run_repair(&repo, repo_id)?;

    assert!(report.findings.is_empty());
    assert_eq!(table_snapshot(repo.local_db.as_ref())?, before);
    assert!(!artifact_root.exists());
    Ok(())
}

#[test]
fn repair_dry_run_reports_preparing_without_recovery_write() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger, 8, Some("notes"), Some("urn:notes"))?;
    let repo_id = repo.get_repo_info()?.expect("RepoInfo").uuid;
    let store = RemoteImportStore::open(repo.local_db.clone(), repo_id)?;
    let preparing = store.reserve(
        RemoteImportDigest::of(b"source"),
        RemoteImportDigest::of(b"locator"),
        0.into(),
        RemoteImportDigest::of(b"ignore"),
    )?;
    let before = table_snapshot(repo.local_db.as_ref())?;
    let artifact_root = crate::utils::notegit::host_dir(&ledger)
        .join("remote-imports")
        .join(repo_id.to_string());

    let report = RemoteImportRuntime::dry_run_repair(&repo, repo_id)?;

    assert!(
        report
            .findings
            .contains(&RemoteImportRepairFinding::Interrupted(
                preparing.session_id
            ))
    );
    assert_eq!(table_snapshot(repo.local_db.as_ref())?, before);
    assert!(!artifact_root.exists());
    Ok(())
}

#[test]
fn empty_store_rejects_zero_next_counters_before_reserve() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let runtime = crate::remote_import::types::RemoteImportRuntimeRecord {
        next_generation: 0,
        next_order: 0,
        ..Default::default()
    };
    let bytes = crate::codec::encode(&runtime)?;
    let write = fixture.db.begin_write()?;
    {
        let mut table = write.open_table(REMOTE_IMPORT_RUNTIME)?;
        table.insert(&0, bytes.as_slice())?;
    }
    write.commit()?;

    let error = RemoteImportStore::open(fixture.db.clone(), fixture.repo_id)
        .err()
        .expect("zero next counters must fail closed");

    assert!(
        matches!(error, RemoteImportError::Storage(message) if message.contains("next generation/order"))
    );
    Ok(())
}

#[test]
fn store_open_rejects_zero_candidate_revision() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    let mut ready = capture.finish()?;
    ready.candidate.as_mut().expect("candidate").revision =
        RemoteImportCandidateRevision::from_u64_for_test(0);
    let bytes = crate::codec::encode(&ready)?;
    let write = fixture.db.begin_write()?;
    {
        let mut sessions = write.open_table(REMOTE_IMPORT_SESSIONS)?;
        sessions.insert(&session_id.as_u128(), bytes.as_slice())?;
    }
    write.commit()?;

    let error = RemoteImportStore::open_read_only(fixture.db.clone(), fixture.repo_id)
        .err()
        .expect("zero candidate revision must fail closed");

    assert!(
        matches!(error, RemoteImportError::Storage(message) if message.contains("candidate revision"))
    );
    Ok(())
}

#[test]
fn abandoned_capture_fails_immediately_and_can_be_discarded() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let capture = fixture.runtime.begin_prepare(fixture.request())?;
    let session_id = capture.session_id();
    drop(capture);

    let record = fixture.runtime.session(session_id)?;
    assert_eq!(record.state, RemoteImportState::Failed);
    let failure = record.failure.expect("interrupted failure");
    assert_eq!(failure.phase, RemoteImportFailurePhase::Capture);
    assert_eq!(failure.kind, RemoteImportFailureKind::Interrupted);

    fixture.runtime.discard(session_id, None)?;
    fixture.runtime.begin_prepare(fixture.request())?.abort()?;
    Ok(())
}

#[test]
fn prepare_keeps_remote_binding_distinct_from_local_projection_locator() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut request = fixture.request();
    request.baseline.locator_digest = RemoteImportDigest::of(b"other-locator");

    let capture = fixture.runtime.begin_prepare(request)?;
    let record = fixture.runtime.session(capture.session_id())?;
    assert_ne!(
        record.locator_binding_digest,
        RemoteImportDigest::of(b"other-locator")
    );
    Ok(())
}

#[test]
fn remote_paths_reject_host_reserved_names_and_characters() {
    for path in [
        "CON.md",
        "aux/readme.md",
        "docs/LPT9.markdown",
        "bad:name.md",
        "bad?.md",
        "control\u{001f}.md",
    ] {
        assert!(
            matches!(
                validate_remote_path(path),
                Err(RemoteImportError::InvalidPath { .. })
            ),
            "reserved path unexpectedly accepted: {path:?}"
        );
    }
    assert!(validate_remote_path("content/concept.md").is_ok());
}

#[test]
fn source_read_failure_is_not_misclassified_as_artifact_io() -> anyhow::Result<()> {
    struct BrokenReader;
    impl Read for BrokenReader {
        fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
            Err(IoError::other("provider stream failed"))
        }
    }

    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    let session_id = capture.session_id();
    assert!(matches!(
        capture.capture_file("note.md", BrokenReader),
        Err(RemoteImportError::SourceRead(_))
    ));
    assert_eq!(
        fixture
            .runtime
            .session(session_id)?
            .failure
            .expect("failure")
            .kind,
        RemoteImportFailureKind::SourceRead
    );
    Ok(())
}

#[test]
fn refresh_rebinds_current_local_projection_locator_from_sealed_blobs() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;

    let refreshed = fixture.runtime.refresh_from_sealed(
        session_id,
        fixture.refresh_request(
            RemoteImportCandidateRevision::FIRST,
            RemoteImportBaseline {
                ledger_head: 8.into(),
                ignore_digest: RemoteImportDigest::of(b"ignore"),
                locator_digest: RemoteImportDigest::of(b"different"),
                existing: BTreeMap::new(),
            },
        ),
    )?;

    assert_eq!(
        refreshed.candidate.expect("candidate").locator_digest,
        RemoteImportDigest::of(b"different")
    );
    assert!(
        fixture
            .artifact_session(session_id)
            .join(CANDIDATES_DIR)
            .join("2.json")
            .exists()
    );
    Ok(())
}

#[test]
fn source_binding_drift_cannot_rebind_sealed_session() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;

    let error = fixture
        .runtime
        .refresh_from_sealed(
            session_id,
            RemoteImportRefreshRequest {
                expected_revision: RemoteImportCandidateRevision::FIRST,
                source_binding_digest: RemoteImportDigest::of(b"different-source-profile"),
                locator_binding_digest: RemoteImportDigest::of(b"remote-locator"),
                baseline: RemoteImportBaseline {
                    ledger_head: 8.into(),
                    ignore_digest: RemoteImportDigest::of(b"ignore"),
                    locator_digest: RemoteImportDigest::of(b"local-locator"),
                    existing: BTreeMap::new(),
                },
            },
        )
        .expect_err("source/profile drift must not rebind sealed blobs");

    assert!(matches!(error, RemoteImportError::ArtifactTampered(_)));
    assert_eq!(
        fixture.runtime.session(session_id)?.state,
        RemoteImportState::Stale
    );
    assert!(
        !fixture
            .artifact_session(session_id)
            .join(CANDIDATES_DIR)
            .join("2.json")
            .exists()
    );
    Ok(())
}

#[test]
fn remote_locator_binding_drift_cannot_rebind_sealed_session() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;

    let error = fixture
        .runtime
        .refresh_from_sealed(
            session_id,
            RemoteImportRefreshRequest {
                expected_revision: RemoteImportCandidateRevision::FIRST,
                source_binding_digest: RemoteImportDigest::of(b"source"),
                locator_binding_digest: RemoteImportDigest::of(b"different-remote-locator"),
                baseline: RemoteImportBaseline {
                    ledger_head: 8.into(),
                    ignore_digest: RemoteImportDigest::of(b"ignore"),
                    locator_digest: RemoteImportDigest::of(b"local-locator-2"),
                    existing: BTreeMap::new(),
                },
            },
        )
        .expect_err("remote locator/profile drift must not rebind sealed blobs");

    assert!(matches!(error, RemoteImportError::ArtifactTampered(_)));
    assert_eq!(
        fixture.runtime.session(session_id)?.state,
        RemoteImportState::Stale
    );
    Ok(())
}

#[test]
fn failure_transition_is_idempotent_and_never_downgrades_reason() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let store = RemoteImportStore::open(fixture.db.clone(), fixture.repo_id)?;
    let request = fixture.request();
    let record = store.reserve(
        request.source_binding_digest,
        request.locator_binding_digest,
        request.baseline.ledger_head,
        request.baseline.ignore_digest,
    )?;
    let source_failure = RemoteImportFailure {
        phase: RemoteImportFailurePhase::Capture,
        kind: RemoteImportFailureKind::SourceRead,
    };

    let failed = store.fail(&record, source_failure.clone())?;
    assert_eq!(store.fail(&record, source_failure)?, failed);
    let downgrade = store
        .fail(
            &record,
            RemoteImportFailure {
                phase: RemoteImportFailurePhase::Capture,
                kind: RemoteImportFailureKind::Interrupted,
            },
        )
        .expect_err("persisted typed failure must not be overwritten");
    assert!(matches!(downgrade, RemoteImportError::InvalidState { .. }));
    assert_eq!(fixture.runtime.session(record.session_id)?, failed);
    Ok(())
}

type RemoteImportTableSnapshot = (Vec<(u128, Vec<u8>)>, Vec<(u8, Vec<u8>)>);

fn table_snapshot(db: &redb::Database) -> anyhow::Result<RemoteImportTableSnapshot> {
    let read = db.begin_read()?;
    let sessions = read.open_table(REMOTE_IMPORT_SESSIONS)?;
    let mut session_rows = Vec::new();
    for row in sessions.iter()? {
        let (key, value) = row?;
        session_rows.push((key.value(), value.value().to_vec()));
    }
    session_rows.sort_by_key(|(key, _)| *key);
    let runtime = read.open_table(REMOTE_IMPORT_RUNTIME)?;
    let mut runtime_rows = Vec::new();
    for row in runtime.iter()? {
        let (key, value) = row?;
        runtime_rows.push((key.value(), value.value().to_vec()));
    }
    runtime_rows.sort_by_key(|(key, _)| *key);
    Ok((session_rows, runtime_rows))
}
