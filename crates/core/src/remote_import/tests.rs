//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 06_backup#remote-import-state-machine
//!   - 03_storage/repair#remote-import-cleanup-repair

mod apply;
mod repair_observation;
mod retention;
mod review_regressions;

use super::artifact::{MANIFEST_FILE, RemoteImportArtifactRoot};
use super::error::RemoteImportError;
use super::repair::RemoteImportRepairFinding;
use super::runtime::RemoteImportRuntime;
use super::store::RemoteImportStore;
use super::types::{
    RemoteImportBaseline, RemoteImportCandidateRevision, RemoteImportDigest,
    RemoteImportFailureKind, RemoteImportPrepareRequest, RemoteImportRefreshRequest,
    RemoteImportState,
};
use crate::ledger::schema::{
    PROJECTION_FAULTS, REDB_SCHEMA_VERSION, REMOTE_IMPORT_RUNTIME, REMOTE_IMPORT_SESSIONS,
};
use crate::ledger::{RepoManager, init::RepoInitOptions};
use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct Fixture {
    _dir: tempfile::TempDir,
    ledger: PathBuf,
    repo_id: crate::models::RepoId,
    repo: RepoManager,
    db: Arc<redb::Database>,
    runtime: RemoteImportRuntime,
}

impl Fixture {
    fn new() -> anyhow::Result<Self> {
        let dir = tempfile::tempdir()?;
        let ledger = dir.path().join("ledger");
        let repo_id = uuid::Uuid::new_v4();
        let execution_name = repo_id.to_string();
        let repo = RepoManager::init_with_options(
            &ledger,
            8,
            Some(&execution_name),
            RepoInitOptions {
                repo_id: Some(repo_id),
                repo_url: None,
            },
        )?;
        let projection_base = dir.path().join("projection");
        let locator =
            repo.prepare_projection_locator_for_repo_creation(repo_id, &projection_base)?;
        let workspace = locator.projection_base_abs.join(&locator.workspace_segment);
        std::fs::create_dir_all(&workspace)?;
        crate::utils::notegit::ensure_repo_identity_marker(&workspace, repo_id, &execution_name)?;
        repo.seed_catalog_membership_from_records()?;
        let authority = repo.claim_repo_catalog_cut_authority()?;
        let prepared = repo.prepare_repo_creation_membership(repo_id, uuid::Uuid::new_v4())?;
        let revalidated = repo.revalidate_repo_creation_membership(&prepared)?;
        let permit = authority.permit(repo_id)?;
        repo.commit_repo_creation_membership(&prepared, &revalidated, &permit)?;
        let handle = repo.open_database(None, repo.local_repo_name())?;
        let db = handle.db;
        let runtime = RemoteImportRuntime::open(&repo, repo_id)?;
        Ok(Self {
            _dir: dir,
            ledger,
            repo_id,
            repo,
            db,
            runtime,
        })
    }

    fn request(&self) -> RemoteImportPrepareRequest {
        RemoteImportPrepareRequest {
            source_binding_digest: RemoteImportDigest::of(b"source"),
            locator_binding_digest: RemoteImportDigest::of(b"remote-locator"),
            baseline: RemoteImportBaseline {
                ledger_head: 7.into(),
                ignore_digest: RemoteImportDigest::of(b"ignore"),
                locator_digest: RemoteImportDigest::of(b"local-locator"),
                existing: BTreeMap::new(),
            },
        }
    }

    fn artifact_session(&self, session_id: super::types::RemoteImportSessionId) -> PathBuf {
        crate::utils::notegit::host_dir(&self.ledger)
            .join("remote-imports")
            .join(self.repo_id.to_string())
            .join(session_id.to_string())
    }

    fn refresh_request(
        &self,
        expected_revision: super::types::RemoteImportCandidateRevision,
        baseline: RemoteImportBaseline,
    ) -> RemoteImportRefreshRequest {
        RemoteImportRefreshRequest {
            expected_revision,
            source_binding_digest: RemoteImportDigest::of(b"source"),
            locator_binding_digest: RemoteImportDigest::of(b"remote-locator"),
            baseline,
        }
    }

    fn repair(&self) -> super::error::RemoteImportResult<super::repair::RemoteImportRepairReport> {
        RemoteImportRuntime::dry_run_repair(&self.repo, self.repo_id)
    }
}

#[test]
fn redb_v4_local_repo_uses_uuid_stem_and_remote_import_tables() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    assert_eq!(REDB_SCHEMA_VERSION, 4);
    assert!(
        fixture
            .ledger
            .join("local")
            .join(format!("{}.redb", fixture.repo_id))
            .is_file()
    );
    let read = fixture.db.begin_read()?;
    read.open_table(REMOTE_IMPORT_SESSIONS)?;
    read.open_table(REMOTE_IMPORT_RUNTIME)?;
    read.open_table(PROJECTION_FAULTS)?;
    Ok(())
}

#[test]
fn prepare_publishes_immutable_ready_session_and_blocks_second_active() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("notes/a.md", Cursor::new(b"alpha"))?;
    capture.capture_file("notes/b.markdown", Cursor::new(b"beta"))?;
    let session_id = capture.session_id();
    let record = capture.finish()?;

    assert_eq!(record.state, RemoteImportState::Ready);
    assert_eq!(
        record.source_snapshot.as_ref().expect("source").file_count,
        2
    );
    assert!(fixture.artifact_session(session_id).is_dir());
    assert!(matches!(
        fixture.runtime.begin_prepare(fixture.request()),
        Err(RemoteImportError::ActiveSession(id)) if id == session_id
    ));
    Ok(())
}

#[test]
fn capture_rejects_unsafe_duplicate_and_oversized_inputs() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    let session_id = capture.session_id();
    let error = capture
        .capture_file("../escape.md", Cursor::new(b"x"))
        .expect_err("traversal must fail");
    assert!(matches!(error, RemoteImportError::InvalidPath { .. }));
    let record = fixture.runtime.session(session_id)?;
    assert_eq!(record.state, RemoteImportState::Failed);
    assert_eq!(
        record.failure.expect("failure").kind,
        RemoteImportFailureKind::InvalidPath
    );

    fixture.runtime.discard(session_id, None)?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    let too_large = vec![0u8; super::artifact::MAX_FILE_PAYLOAD_BYTES as usize + 1];
    let error = capture
        .capture_file("large.md", Cursor::new(too_large))
        .expect_err("oversized file must fail");
    assert!(matches!(error, RemoteImportError::LimitExceeded { .. }));
    Ok(())
}

#[test]
fn startup_recovers_preparing_as_failed_without_source_replay() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let capture = fixture.runtime.begin_prepare(fixture.request())?;
    let session_id = capture.session_id();
    std::mem::forget(capture);

    RemoteImportRuntime::recover_startup(&fixture.repo, fixture.repo_id)?;
    let reopened = RemoteImportRuntime::open(&fixture.repo, fixture.repo_id)?;
    let record = reopened.session(session_id)?;
    assert_eq!(record.state, RemoteImportState::Failed);
    assert_eq!(
        record.failure.expect("failure").kind,
        RemoteImportFailureKind::Interrupted
    );
    Ok(())
}

#[test]
fn ordinary_open_does_not_recover_current_process_capture() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let capture = fixture.runtime.begin_prepare(fixture.request())?;
    let session_id = capture.session_id();

    let reopened = RemoteImportRuntime::open(&fixture.repo, fixture.repo_id)?;
    assert_eq!(
        reopened.session(session_id)?.state,
        RemoteImportState::Preparing
    );
    capture.abort()?;
    Ok(())
}

#[test]
fn refresh_uses_sealed_blobs_and_tamper_is_reported() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    let ready = capture.finish()?;
    let refreshed = fixture.runtime.refresh_from_sealed(
        session_id,
        fixture.refresh_request(
            RemoteImportCandidateRevision::FIRST,
            RemoteImportBaseline {
                ledger_head: 9.into(),
                ignore_digest: RemoteImportDigest::of(b"ignore-2"),
                locator_digest: RemoteImportDigest::of(b"local-locator-2"),
                existing: BTreeMap::new(),
            },
        ),
    )?;
    assert_eq!(
        refreshed
            .candidate
            .as_ref()
            .expect("candidate")
            .revision
            .get(),
        2
    );

    let manifest = fixture.artifact_session(session_id).join("blobs");
    let blob = std::fs::read_dir(manifest)?.next().expect("blob")?.path();
    std::fs::write(blob, b"tampered")?;
    let report = fixture.repair()?;
    assert!(
        report
            .findings
            .contains(&RemoteImportRepairFinding::ArtifactTamper(session_id))
    );
    assert_eq!(ready.session_id, session_id);
    Ok(())
}

#[test]
fn discard_cleans_only_exact_session_and_clears_active_pointer() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;
    let unrelated = crate::utils::notegit::host_dir(&fixture.ledger).join("keep.txt");
    std::fs::write(&unrelated, b"keep")?;

    let discarded = fixture
        .runtime
        .discard(session_id, Some(RemoteImportCandidateRevision::FIRST))?;
    assert_eq!(discarded.state, RemoteImportState::Discarded);
    assert!(!discarded.cleanup_pending);
    assert!(!fixture.artifact_session(session_id).exists());
    assert_eq!(std::fs::read(unrelated)?, b"keep");
    let _next = fixture.runtime.begin_prepare(fixture.request())?;
    Ok(())
}

#[test]
fn cleanup_pending_records_are_never_pruned() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
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
            super::types::RemoteImportFailure {
                phase: super::types::RemoteImportFailurePhase::Capture,
                kind: RemoteImportFailureKind::SourceRead,
            },
        )?;
        store.begin_discard(record.session_id, record.generation, None)?;
    }
    let records = store.list_sessions()?;
    assert_eq!(records.len(), 65);
    assert!(records.iter().all(|record| record.cleanup_pending));
    Ok(())
}

#[test]
fn manifest_bytes_are_deterministic_across_provider_listing_order() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut first = fixture.runtime.begin_prepare(fixture.request())?;
    first.capture_file("z.md", Cursor::new(b"zulu"))?;
    first.capture_file("a.md", Cursor::new(b"alpha"))?;
    let first_id = first.session_id();
    first.finish()?;
    let first_bytes = std::fs::read(fixture.artifact_session(first_id).join(MANIFEST_FILE))?;
    fixture
        .runtime
        .discard(first_id, Some(RemoteImportCandidateRevision::FIRST))?;

    let mut second = fixture.runtime.begin_prepare(fixture.request())?;
    second.capture_file("a.md", Cursor::new(b"alpha"))?;
    second.capture_file("z.md", Cursor::new(b"zulu"))?;
    let second_id = second.session_id();
    second.finish()?;
    let second_bytes = std::fs::read(fixture.artifact_session(second_id).join(MANIFEST_FILE))?;

    assert_eq!(first_bytes, second_bytes);
    Ok(())
}

#[test]
fn capture_rejects_case_insensitive_path_collisions() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("Notes/A.md", Cursor::new(b"first"))?;
    let error = capture
        .capture_file("notes/a.md", Cursor::new(b"second"))
        .expect_err("case-insensitive aliases must not share one snapshot");
    assert!(matches!(error, RemoteImportError::DuplicatePath(path) if path == "notes/a.md"));
    Ok(())
}

#[test]
fn eligible_terminal_retention_keeps_exactly_latest_64() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
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
            super::types::RemoteImportFailure {
                phase: super::types::RemoteImportFailurePhase::Capture,
                kind: RemoteImportFailureKind::SourceRead,
            },
        )?;
        store.begin_discard(record.session_id, record.generation, None)?;
        store.finish_cleanup(record.session_id)?;
    }
    let records = store.list_sessions()?;
    assert_eq!(records.len(), super::store::retention::TERMINAL_RETENTION);
    assert_eq!(records.first().expect("oldest retained").order, 2);
    assert_eq!(records.last().expect("newest retained").order, 65);
    Ok(())
}

#[test]
fn session_rows_reject_trailing_postcard_bytes() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let store = RemoteImportStore::open(fixture.db.clone(), fixture.repo_id)?;
    let record = store.reserve(
        RemoteImportDigest::of(b"source"),
        RemoteImportDigest::of(b"locator"),
        1.into(),
        RemoteImportDigest::of(b"ignore"),
    )?;
    let mut bytes = {
        let read = fixture.db.begin_read()?;
        let table = read.open_table(REMOTE_IMPORT_SESSIONS)?;
        table
            .get(&record.session_id.as_u128())?
            .expect("session row")
            .value()
            .to_vec()
    };
    bytes.push(0);
    let write = fixture.db.begin_write()?;
    {
        let mut table = write.open_table(REMOTE_IMPORT_SESSIONS)?;
        table.insert(&record.session_id.as_u128(), bytes.as_slice())?;
    }
    write.commit()?;

    assert!(matches!(
        store.read_session(record.session_id),
        Err(RemoteImportError::Codec(message)) if message.contains("trailing bytes")
    ));
    Ok(())
}

fn durable_bytes(
    db: &redb::Database,
    session_id: super::types::RemoteImportSessionId,
) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let read = db.begin_read()?;
    let sessions = read.open_table(REMOTE_IMPORT_SESSIONS)?;
    let session = sessions
        .get(&session_id.as_u128())?
        .expect("session")
        .value()
        .to_vec();
    let runtime = read.open_table(REMOTE_IMPORT_RUNTIME)?;
    let runtime = runtime.get(&0)?.expect("runtime").value().to_vec();
    Ok((session, runtime))
}

fn artifact_tree_bytes(root: &Path) -> anyhow::Result<Vec<(PathBuf, Option<Vec<u8>>)>> {
    fn walk(
        root: &Path,
        current: &Path,
        entries: &mut Vec<(PathBuf, Option<Vec<u8>>)>,
    ) -> anyhow::Result<()> {
        let mut children = std::fs::read_dir(current)?
            .map(|entry| entry.map(|value| value.path()))
            .collect::<Result<Vec<_>, _>>()?;
        children.sort();
        for child in children {
            let relative = child.strip_prefix(root)?.to_path_buf();
            if child.is_dir() {
                entries.push((relative, None));
                walk(root, &child, entries)?;
            } else {
                entries.push((relative, Some(std::fs::read(child)?)));
            }
        }
        Ok(())
    }

    let mut entries = Vec::new();
    walk(root, root, &mut entries)?;
    Ok(entries)
}
