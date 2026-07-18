//! plan_ref:
//!   - 03_storage/authority#sealed-ledger-change-batch
//!   - 06_backup#remote-import-session-contract

mod admission;
mod replay;

use super::Fixture;
use crate::ledger::{range, reconcile};
use crate::models::Op;
use crate::remote_import::{
    RemoteImportApplyRequest, RemoteImportBlocker, RemoteImportCandidateRevision,
    RemoteImportDigest, RemoteImportError, RemoteImportProjectionOutcome, RemoteImportState,
};
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use std::io::Cursor;

fn ready_session(
    fixture: &Fixture,
    path: &str,
    content: &[u8],
) -> anyhow::Result<crate::remote_import::RemoteImportSessionRecord> {
    let mut prepare = fixture.request();
    prepare.baseline.ledger_head = current_head(fixture)?.into();
    let mut capture = fixture.runtime.begin_prepare(prepare)?;
    capture.capture_file(path, Cursor::new(content))?;
    Ok(capture.finish()?)
}

fn apply_request(
    record: &crate::remote_import::RemoteImportSessionRecord,
) -> RemoteImportApplyRequest {
    let candidate = record.candidate.as_ref().expect("ready candidate");
    RemoteImportApplyRequest {
        request_id: uuid::Uuid::new_v4(),
        session_id: record.session_id,
        revision: candidate.revision,
        locator_digest: candidate.locator_digest,
        ignore_digest: candidate.ignore_digest,
    }
}

fn current_head(fixture: &Fixture) -> anyhow::Result<u64> {
    range::get_max_seq(fixture.db.as_ref())
}

fn seed_local_doc(
    fixture: &Fixture,
    path: &str,
    content: &str,
) -> anyhow::Result<crate::models::DocId> {
    let (doc_id, _) = fixture.repo.apply_file_structure_in_local_repo(
        fixture.repo.local_repo_name(),
        path,
        None,
        "test seed",
    )?;
    reconcile::append_patch_in_local_repo(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        doc_id,
        "test seed",
        &[Op::Insert {
            pos: 0,
            content: content.into(),
        }],
    )?;
    Ok(doc_id)
}

#[test]
fn sealed_apply_commits_whole_session_and_replays_stored_receipt() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = ready_session(&fixture, "notes/imported.md", b"alpha")?;
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

    assert_eq!(receipt.request_id, request.request_id);
    assert_eq!(receipt.authority_head_before.storage_key(), 0);
    assert!(receipt.authority_head_after.storage_key() > 0);
    assert_eq!(
        receipt.projection_outcome,
        RemoteImportProjectionOutcome::Pending
    );
    let applied = fixture.runtime.session(ready.session_id)?;
    assert_eq!(applied.state, RemoteImportState::Applied);
    assert_eq!(applied.apply_receipt.as_ref(), Some(&receipt));
    assert!(applied.cleanup_pending);
    let doc_id = fixture
        .repo
        .get_tracked_docid_in_local_repo(fixture.repo.local_repo_name(), "notes/imported.md")?
        .expect("imported doc");
    let entries = fixture
        .repo
        .get_local_ops_in_local_repo(fixture.repo.local_repo_name(), doc_id)?
        .into_iter()
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();
    assert_eq!(crate::state::reconstruct_content(&entries), "alpha");
    let committed_head = current_head(&fixture)?;

    std::fs::remove_dir_all(fixture.artifact_session(ready.session_id))?;
    let replay = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        request.clone(),
    )?;
    let replayed =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), replay)?;
    assert_eq!(replayed, receipt);
    assert_eq!(current_head(&fixture)?, committed_head);

    let mut different_request = request;
    different_request.request_id = uuid::Uuid::new_v4();
    assert!(matches!(
        fixture.runtime.prepare_apply(
            &fixture.repo,
            fixture.repo.local_repo_name(),
            different_request
        ),
        Err(RemoteImportError::InvalidState { .. })
    ));
    Ok(())
}

#[test]
fn apply_head_drift_persists_stale_without_fact_prefix() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = ready_session(&fixture, "notes/imported.md", b"alpha")?;
    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        apply_request(&ready),
    )?;
    fixture.repo.apply_dir_create_structure_in_local_repo(
        fixture.repo.local_repo_name(),
        "unrelated",
        "test concurrent fact",
    )?;
    let concurrent_head = current_head(&fixture)?;

    assert!(matches!(
        fixture.runtime.commit_apply(
            &fixture.repo,
            fixture.repo.local_repo_name(),
            prepared
        ),
        Err(RemoteImportError::Stale { blockers, .. })
            if blockers == vec![RemoteImportBlocker::LedgerHeadDrift]
    ));
    assert_eq!(current_head(&fixture)?, concurrent_head);
    assert!(
        fixture
            .repo
            .get_tracked_docid_in_local_repo(fixture.repo.local_repo_name(), "notes/imported.md")?
            .is_none()
    );
    let stale = fixture.runtime.session(ready.session_id)?;
    assert_eq!(stale.state, RemoteImportState::Stale);
    assert!(stale.apply_receipt.is_none());
    Ok(())
}

#[test]
fn pending_and_staged_overlap_block_without_consuming_or_appending() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = ready_session(&fixture, "notes/imported.md", b"alpha")?;
    let request = apply_request(&ready);
    let pending = PendingFsEntry {
        path: "notes/imported.md".to_string(),
        renamed_from: None,
        doc_id: None,
        change_type: ChangeStatus::Added,
        content_hash: pending_fs::content_hash("local"),
        detected_at: 1,
        has_conflict: false,
    };

    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        request.clone(),
    )?;
    pending_fs::upsert(fixture.db.as_ref(), &pending)?;
    assert!(matches!(
        fixture.runtime.commit_apply(
            &fixture.repo,
            fixture.repo.local_repo_name(),
            prepared
        ),
        Err(RemoteImportError::Blocked { blockers, .. })
            if blockers == vec![RemoteImportBlocker::PendingOverlap]
    ));
    assert_eq!(current_head(&fixture)?, 0);
    pending_fs::remove(fixture.db.as_ref(), &pending.path)?;

    let prepared =
        fixture
            .runtime
            .prepare_apply(&fixture.repo, fixture.repo.local_repo_name(), request)?;
    pending_fs::upsert(fixture.db.as_ref(), &pending)?;
    crate::source_control::staging::stage_pending_entries_atomically(
        fixture.db.as_ref(),
        std::slice::from_ref(&pending),
        false,
    )?;
    assert!(matches!(
        fixture.runtime.commit_apply(
            &fixture.repo,
            fixture.repo.local_repo_name(),
            prepared
        ),
        Err(RemoteImportError::Blocked { blockers, .. })
            if blockers == vec![RemoteImportBlocker::StagedOverlap]
    ));
    assert_eq!(current_head(&fixture)?, 0);
    assert_eq!(
        fixture.runtime.session(ready.session_id)?.state,
        RemoteImportState::Ready
    );
    assert!(
        crate::source_control::staging::get_staged(fixture.db.as_ref(), &pending.path)?.is_some()
    );
    Ok(())
}

#[test]
fn apply_rejects_tampered_blob_before_authority_write() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = ready_session(&fixture, "notes/imported.md", b"alpha")?;
    let blob = fixture
        .artifact_session(ready.session_id)
        .join(super::super::artifact::BLOBS_DIR)
        .join(RemoteImportDigest::of(b"alpha").to_hex());
    std::fs::write(blob, b"bravo")?;
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
    assert!(fixture.runtime.session(ready.session_id)?.failure.is_some());
    Ok(())
}

#[test]
fn whole_session_failure_rolls_back_an_already_appended_prefix() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut prepare = fixture.request();
    prepare.baseline.ledger_head = 0.into();
    let mut capture = fixture.runtime.begin_prepare(prepare)?;
    capture.capture_file("a.md", Cursor::new(b"parent file"))?;
    capture.capture_file("a.md/child.md", Cursor::new(b"child"))?;
    let ready = capture.finish()?;
    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        apply_request(&ready),
    )?;

    assert!(matches!(
        fixture.runtime.commit_apply(
            &fixture.repo,
            fixture.repo.local_repo_name(),
            prepared
        ),
        Err(RemoteImportError::ApplyFailed(message))
            if message.contains("target parent is not a directory")
    ));
    assert_eq!(current_head(&fixture)?, 0);
    assert!(
        fixture
            .repo
            .get_tracked_docid_in_local_repo(fixture.repo.local_repo_name(), "a.md")?
            .is_none()
    );
    assert_eq!(
        fixture.runtime.session(ready.session_id)?.state,
        RemoteImportState::Ready
    );
    Ok(())
}

#[test]
fn modified_and_unchanged_candidates_preserve_exact_head_semantics() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let doc_id = seed_local_doc(&fixture, "notes/existing.md", "old")?;
    let mut prepare = fixture.request();
    prepare.baseline.ledger_head = current_head(&fixture)?.into();
    prepare.baseline.existing.insert(
        "notes/existing.md".to_string(),
        RemoteImportDigest::of(b"old"),
    );
    let mut capture = fixture.runtime.begin_prepare(prepare)?;
    capture.capture_file("notes/existing.md", Cursor::new(b"new"))?;
    let modified = capture.finish()?;
    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        apply_request(&modified),
    )?;
    fixture
        .runtime
        .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), prepared)?;
    let content = fixture
        .repo
        .get_local_ops_in_local_repo(fixture.repo.local_repo_name(), doc_id)?
        .into_iter()
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();
    assert_eq!(crate::state::reconstruct_content(&content), "new");

    let head_before_unchanged = current_head(&fixture)?;
    let mut prepare = fixture.request();
    prepare.baseline.ledger_head = head_before_unchanged.into();
    prepare.baseline.existing.insert(
        "notes/existing.md".to_string(),
        RemoteImportDigest::of(b"new"),
    );
    let mut capture = fixture.runtime.begin_prepare(prepare)?;
    capture.capture_file("notes/existing.md", Cursor::new(b"new"))?;
    let unchanged = capture.finish()?;
    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        apply_request(&unchanged),
    )?;
    let receipt =
        fixture
            .runtime
            .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), prepared)?;
    assert_eq!(
        receipt.authority_head_before.storage_key(),
        head_before_unchanged
    );
    assert_eq!(
        receipt.authority_head_after.storage_key(),
        head_before_unchanged
    );
    assert_eq!(current_head(&fixture)?, head_before_unchanged);
    Ok(())
}

#[test]
fn remote_absence_never_deletes_local_authority() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let local_doc = seed_local_doc(&fixture, "notes/local-only.md", "keep")?;
    let ready = ready_session(&fixture, "notes/imported.md", b"alpha")?;
    let prepared = fixture.runtime.prepare_apply(
        &fixture.repo,
        fixture.repo.local_repo_name(),
        apply_request(&ready),
    )?;
    fixture
        .runtime
        .commit_apply(&fixture.repo, fixture.repo.local_repo_name(), prepared)?;

    assert_eq!(
        fixture.repo.get_tracked_docid_in_local_repo(
            fixture.repo.local_repo_name(),
            "notes/local-only.md"
        )?,
        Some(local_doc)
    );
    let entries = fixture
        .repo
        .get_local_ops_in_local_repo(fixture.repo.local_repo_name(), local_doc)?
        .into_iter()
        .map(|(_, entry)| entry)
        .collect::<Vec<_>>();
    assert_eq!(crate::state::reconstruct_content(&entries), "keep");
    Ok(())
}

#[test]
fn candidate_revision_type_remains_strong_in_apply_request() {
    assert_eq!(RemoteImportCandidateRevision::FIRST.get(), 1);
}
