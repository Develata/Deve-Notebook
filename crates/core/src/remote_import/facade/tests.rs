//! plan_ref:
//!   - 06_backup#remote-import-session-contract
//!   - 05_diff_logic#remote-import-diff-contract

use super::*;
use crate::remote_import::RemoteImportBlocker;
use crate::remote_import::RemoteImportState;
use crate::remote_import::types::{RemoteImportApplyReceipt, RemoteImportProjectionOutcome};
use crate::source_control::ChangeStatus;
use crate::source_control::pending_fs::{self, PendingFsEntry};
use crate::source_control::staging;
use std::io::Cursor;
use std::sync::Arc;

struct Fixture {
    _dir: tempfile::TempDir,
    repo: Arc<RepoManager>,
    repo_id: RepoId,
    source: RemoteImportBinding,
    locator: RemoteImportBinding,
}

impl Fixture {
    fn new() -> anyhow::Result<Self> {
        let dir = tempfile::tempdir()?;
        let projection_base = dir.path().join("notes");
        let (repo, repo_id) = crate::test_support::init_cataloged_repo_with_url(
            &dir.path().join("ledger"),
            &projection_base,
            "webdav+https://dav.example.com/notebooks/main",
        )?;
        let repo = Arc::new(repo);
        Ok(Self {
            _dir: dir,
            repo,
            repo_id,
            source: RemoteImportBinding::from_canonical_identity("source", b"webdav"),
            locator: RemoteImportBinding::from_canonical_identity(
                "locator-profile",
                b"webdav+https://dav.example.com/notebooks/main",
            ),
        })
    }

    fn ready_session(&self) -> anyhow::Result<RemoteImportSessionView> {
        let service = RemoteImportService::open(self.repo.as_ref(), self.repo_id)?;
        let mut capture = service.begin_prepare(
            self.repo.as_ref(),
            self.repo.local_repo_name(),
            &self.source,
            &self.locator,
        )?;
        capture.capture_file("notes/imported.md", Cursor::new(b"remote"))?;
        Ok(capture.finish()?)
    }

    fn pending(&self) -> PendingFsEntry {
        PendingFsEntry {
            path: "notes/imported.md".to_string(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("local"),
            detected_at: 1,
            has_conflict: false,
        }
    }

    fn put_pending(&self, entry: &PendingFsEntry) -> anyhow::Result<()> {
        self.repo
            .run_on_local_repo(self.repo.local_repo_name(), |db| {
                pending_fs::upsert(db, entry)
            })?;
        Ok(())
    }
}

#[test]
fn pending_overlap_is_whole_session_blocker_before_apply() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = fixture.ready_session()?;
    fixture.put_pending(&fixture.pending())?;
    let service = RemoteImportService::open(fixture.repo.as_ref(), fixture.repo_id)?;
    let shown = service.show(
        fixture.repo.as_ref(),
        fixture.repo.local_repo_name(),
        ready.session_id,
        ready.revision,
        &fixture.locator,
    )?;

    assert_eq!(shown.blockers, vec![RemoteImportBlocker::PendingOverlap]);
    let sync = SyncManager::new_checked(fixture.repo.clone())?;
    assert!(matches!(
        service.apply(
            fixture.repo.as_ref(),
            &sync,
            fixture.repo.local_repo_name(),
            Uuid::new_v4(),
            ready.session_id,
            ready.revision.expect("ready revision"),
            Some(&fixture.locator),
        ),
        Err(RemoteImportError::Blocked { blockers, .. })
            if blockers == vec![RemoteImportBlocker::PendingOverlap]
    ));
    Ok(())
}

#[test]
fn staged_overlap_is_whole_session_review_blocker() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = fixture.ready_session()?;
    let pending = fixture.pending();
    fixture.put_pending(&pending)?;
    fixture
        .repo
        .run_on_local_repo(fixture.repo.local_repo_name(), |db| {
            staging::stage_pending_entries_atomically(db, std::slice::from_ref(&pending), false)
        })?;
    let service = RemoteImportService::open(fixture.repo.as_ref(), fixture.repo_id)?;
    let shown = service.show(
        fixture.repo.as_ref(),
        fixture.repo.local_repo_name(),
        ready.session_id,
        ready.revision,
        &fixture.locator,
    )?;

    assert_eq!(shown.blockers, vec![RemoteImportBlocker::StagedOverlap]);
    Ok(())
}

#[test]
fn local_projection_locator_drift_blocks_apply_until_sealed_refresh_rebinds_it()
-> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = fixture.ready_session()?;
    let relocated = fixture._dir.path().join("relocated");
    std::fs::create_dir_all(&relocated)?;
    fixture
        .repo
        .set_projection_base_for_local_repo(fixture.repo.local_repo_name(), &relocated)?;
    fixture
        .repo
        .ensure_local_repo_workspace_identity(fixture.repo.local_repo_name())?;
    let service = RemoteImportService::open(fixture.repo.as_ref(), fixture.repo_id)?;

    let stale = service.show(
        fixture.repo.as_ref(),
        fixture.repo.local_repo_name(),
        ready.session_id,
        ready.revision,
        &fixture.locator,
    )?;
    assert_eq!(
        stale.blockers,
        vec![RemoteImportBlocker::LocatorBindingDrift]
    );

    let refreshed = service.refresh(
        fixture.repo.as_ref(),
        fixture.repo.local_repo_name(),
        ready.session_id,
        ready.revision.expect("ready revision"),
        &fixture.source,
        &fixture.locator,
    )?;
    let shown = service.show(
        fixture.repo.as_ref(),
        fixture.repo.local_repo_name(),
        ready.session_id,
        refreshed.revision,
        &fixture.locator,
    )?;
    assert!(shown.blockers.is_empty());
    Ok(())
}

#[test]
fn post_commit_settlement_failure_returns_pending_and_marks_health() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let sync = SyncManager::new_checked(fixture.repo.clone())?;
    let pending = RemoteImportApplyReceipt {
        request_id: Uuid::new_v4(),
        session_id: RemoteImportSessionId::from_uuid(Uuid::new_v4()),
        revision: RemoteImportCandidateRevision::from_u64(1),
        writer_peer_id: crate::models::PeerId::new("writer"),
        authority_head_before: 0.into(),
        authority_head_after: 1.into(),
        manifest_digest: super::super::types::RemoteImportDigest::of(b"manifest"),
        candidate_digest: super::super::types::RemoteImportDigest::of(b"candidate"),
        projection_outcome: RemoteImportProjectionOutcome::Pending,
    };

    let observed = super::projection::finish_settlement_for_test(
        &sync,
        fixture.repo.local_repo_name(),
        &pending,
        Err(RemoteImportError::Storage(
            "injected settlement failure".to_string(),
        )),
    );

    assert_eq!(observed, pending);
    assert!(sync.is_projection_degraded(fixture.repo.local_repo_name()));
    Ok(())
}

#[test]
fn pending_projection_artifacts_are_not_repairable_or_deleted() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = fixture.ready_session()?;
    let service = RemoteImportService::open(fixture.repo.as_ref(), fixture.repo_id)?;
    let record = service.inner.session(ready.session_id)?;
    let candidate = record.candidate.as_ref().expect("candidate");
    let prepared = service.inner.prepare_apply(
        fixture.repo.as_ref(),
        fixture.repo.local_repo_name(),
        RemoteImportApplyRequest {
            request_id: Uuid::new_v4(),
            session_id: ready.session_id,
            revision: candidate.revision,
            locator_digest: candidate.locator_digest,
            ignore_digest: candidate.ignore_digest,
        },
    )?;
    let receipt = service.inner.commit_apply(
        fixture.repo.as_ref(),
        fixture.repo.local_repo_name(),
        prepared,
    )?;
    assert_eq!(
        receipt.projection_outcome,
        RemoteImportProjectionOutcome::Pending
    );

    let plan = service.dry_run_repair()?;
    assert_eq!(plan.repairable_count, 0);
    let after = service.apply_repair(plan.token())?;
    assert_eq!(after.repairable_count, 0);
    super::super::artifact::verify_apply_artifacts(&service.inner.artifacts, &record)?;
    assert_eq!(
        service
            .inner
            .session(ready.session_id)?
            .apply_receipt
            .expect("pending receipt")
            .projection_outcome,
        RemoteImportProjectionOutcome::Pending
    );
    let RemoteImportRepoRemovalAdmission::Blocked(blocked) = service.repo_removal_admission()?
    else {
        panic!("Applied/Pending must block repo removal");
    };
    assert_eq!(
        blocked.blockers(),
        &[RemoteImportRepoRemovalBlocker::ProjectionPending {
            session_id: ready.session_id,
        }]
    );
    Ok(())
}

#[test]
fn degraded_projection_settlement_updates_current_process_health() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = fixture.ready_session()?;
    let workspace = fixture
        .repo
        .local_repo_workspace_root(fixture.repo.local_repo_name())?;
    std::fs::create_dir_all(&workspace)?;
    std::fs::write(workspace.join("notes"), b"blocks directory creation")?;
    let sync = SyncManager::new_checked(fixture.repo.clone())?;
    let service = RemoteImportService::open(fixture.repo.as_ref(), fixture.repo_id)?;
    let request_id = Uuid::new_v4();

    let applied = service.apply(
        fixture.repo.as_ref(),
        &sync,
        fixture.repo.local_repo_name(),
        request_id,
        ready.session_id,
        ready.revision.expect("ready revision"),
        Some(&fixture.locator),
    )?;

    assert_eq!(
        applied.projection_outcome,
        RemoteImportProjectionOutcome::Degraded
    );
    assert!(sync.is_projection_degraded(fixture.repo.local_repo_name()));
    let RemoteImportRepoRemovalAdmission::Blocked(blocked) = service.repo_removal_admission()?
    else {
        panic!("Applied/Degraded must block repo removal");
    };
    assert_eq!(
        blocked.blockers(),
        &[RemoteImportRepoRemovalBlocker::ProjectionDegraded {
            session_id: ready.session_id,
        }]
    );
    assert!(service.is_exact_apply_replay(
        fixture.repo.as_ref(),
        request_id,
        ready.session_id,
        ready.revision.expect("ready revision")
    )?);
    let replay = service.apply(
        fixture.repo.as_ref(),
        &sync,
        fixture.repo.local_repo_name(),
        request_id,
        ready.session_id,
        ready.revision.expect("ready revision"),
        None,
    )?;
    assert_eq!(replay, applied);
    Ok(())
}

#[test]
fn optional_revision_none_is_exact_for_precandidate_failure_only() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let service = RemoteImportService::open(fixture.repo.as_ref(), fixture.repo_id)?;

    let mut failed_capture = service.begin_prepare(
        fixture.repo.as_ref(),
        fixture.repo.local_repo_name(),
        &fixture.source,
        &fixture.locator,
    )?;
    let failed_session_id = failed_capture.session_id();
    assert!(
        failed_capture
            .capture_file("../outside.md", Cursor::new(b"unsafe"))
            .is_err()
    );
    drop(failed_capture);

    let failed = service.show(
        fixture.repo.as_ref(),
        fixture.repo.local_repo_name(),
        failed_session_id,
        None,
        &fixture.locator,
    )?;
    assert_eq!(failed.state, RemoteImportState::Failed);
    assert_eq!(failed.revision, None);
    let discarded = service.discard(failed_session_id, None)?;
    assert_eq!(discarded.state, RemoteImportState::Discarded);

    let ready = fixture.ready_session()?;
    assert!(matches!(
        service.show(
            fixture.repo.as_ref(),
            fixture.repo.local_repo_name(),
            ready.session_id,
            None,
            &fixture.locator,
        ),
        Err(RemoteImportError::Stale { .. })
    ));
    assert!(matches!(
        service.discard(ready.session_id, None),
        Err(RemoteImportError::Stale { .. })
    ));

    let revision = ready.revision.expect("ready session revision");
    assert_eq!(
        service
            .show(
                fixture.repo.as_ref(),
                fixture.repo.local_repo_name(),
                ready.session_id,
                Some(revision),
                &fixture.locator,
            )?
            .revision,
        Some(revision)
    );
    assert_eq!(
        service.discard(ready.session_id, Some(revision))?.state,
        RemoteImportState::Discarded
    );
    Ok(())
}

#[test]
fn repo_removal_admission_allows_owned_capture_cleanup_without_hiding_drift() -> anyhow::Result<()>
{
    let fixture = Fixture::new()?;
    let service = RemoteImportService::open(fixture.repo.as_ref(), fixture.repo_id)?;
    let admitted = match service.repo_removal_admission()? {
        RemoteImportRepoRemovalAdmission::Admitted(snapshot) => snapshot,
        other => panic!("empty runtime must admit removal: {other:?}"),
    };
    assert_eq!(
        service.revalidate_repo_removal(&admitted)?,
        RemoteImportRepoRemovalRevalidation::Exact
    );

    let mut preparing = service.begin_prepare(
        fixture.repo.as_ref(),
        fixture.repo.local_repo_name(),
        &fixture.source,
        &fixture.locator,
    )?;
    assert_removal_capture_cleanup(&service)?;
    assert!(matches!(
        service.revalidate_repo_removal(&admitted)?,
        RemoteImportRepoRemovalRevalidation::Changed(RemoteImportRepoRemovalAdmission::Admitted(_))
    ));
    assert!(
        preparing
            .capture_file("../unsafe.md", Cursor::new(b"unsafe"))
            .is_err()
    );
    assert_removal_capture_cleanup(&service)?;
    let failed_id = preparing.session_id();
    drop(preparing);
    service.discard(failed_id, None)?;

    let ready = fixture.ready_session()?;
    assert_removal_capture_cleanup(&service)?;
    service.discard(ready.session_id, ready.revision)?;
    assert!(matches!(
        service.repo_removal_admission()?,
        RemoteImportRepoRemovalAdmission::Admitted(_)
    ));
    Ok(())
}

#[test]
fn repo_removal_admission_marks_owner_cleanup_debt_without_blocking() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let ready = fixture.ready_session()?;
    let service = RemoteImportService::open(fixture.repo.as_ref(), fixture.repo_id)?;
    let record = service.inner.session(ready.session_id)?;
    service.inner.store.begin_discard(
        record.session_id,
        record.generation,
        record
            .candidate
            .as_ref()
            .map(|candidate| candidate.revision),
    )?;

    let RemoteImportRepoRemovalAdmission::Admitted(snapshot) = service.repo_removal_admission()?
    else {
        panic!("owner cleanup debt must remain removable");
    };
    assert_eq!(snapshot.repo_id(), fixture.repo_id);
    assert!(snapshot.capture_cleanup_required());
    Ok(())
}

fn assert_removal_capture_cleanup(service: &RemoteImportService) -> anyhow::Result<()> {
    let RemoteImportRepoRemovalAdmission::Admitted(snapshot) = service.repo_removal_admission()?
    else {
        panic!("owner-cleanable capture must not block repo removal");
    };
    assert!(snapshot.capture_cleanup_required());
    Ok(())
}
