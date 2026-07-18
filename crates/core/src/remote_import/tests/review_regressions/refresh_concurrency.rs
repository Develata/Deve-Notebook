//! plan_ref:
//!   - 06_backup#remote-import-state-machine
//!   - 03_storage/repair#remote-import-cleanup-repair

use super::*;
use crate::remote_import::artifact::{publish_candidate_revision, verify_published_session};
use crate::remote_import::manifest::encode_candidate;
use std::sync::{Arc, Barrier};

#[test]
fn prepublished_exact_refresh_revision_is_retryable_and_repair_visible() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    let ready = capture.finish()?;
    let root = RemoteImportArtifactRoot::open(&fixture.ledger, fixture.repo_id)?;
    let manifest = verify_published_session(&root, &ready)?;
    let baseline = RemoteImportBaseline {
        ledger_head: 9.into(),
        ignore_digest: RemoteImportDigest::of(b"ignore-2"),
        locator_digest: RemoteImportDigest::of(b"local-locator-2"),
        existing: BTreeMap::new(),
    };
    let candidate = encode_candidate(
        &manifest,
        &baseline,
        RemoteImportCandidateRevision::FIRST
            .next()
            .expect("revision 2"),
    )?;
    publish_candidate_revision(&root, &ready, &candidate)?;

    let report = fixture.repair()?;
    assert!(
        report
            .findings
            .contains(&RemoteImportRepairFinding::ExtraCandidate {
                session_id,
                revision: 2,
            })
    );

    let refreshed = fixture.runtime.refresh_from_sealed(
        session_id,
        fixture.refresh_request(RemoteImportCandidateRevision::FIRST, baseline),
    )?;
    assert_eq!(refreshed.candidate.expect("candidate").revision.get(), 2);
    assert!(!fixture.repair()?.findings.iter().any(|finding| {
        matches!(finding, RemoteImportRepairFinding::ExtraCandidate { session_id: id, .. } if *id == session_id)
    }));
    Ok(())
}

#[test]
fn concurrent_refreshes_share_one_exact_revision() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;

    let runtime = Arc::new(RemoteImportRuntime::open(&fixture.repo, fixture.repo_id)?);
    let barrier = Arc::new(Barrier::new(2));
    let baseline = RemoteImportBaseline {
        ledger_head: 11.into(),
        ignore_digest: RemoteImportDigest::of(b"ignore-concurrent"),
        locator_digest: RemoteImportDigest::of(b"locator"),
        existing: BTreeMap::new(),
    };
    let mut workers = Vec::new();
    for _ in 0..2 {
        let runtime = runtime.clone();
        let barrier = barrier.clone();
        let baseline = baseline.clone();
        workers.push(std::thread::spawn(move || {
            runtime.refresh_with_after_read_test(
                session_id,
                RemoteImportRefreshRequest {
                    expected_revision: RemoteImportCandidateRevision::FIRST,
                    source_binding_digest: RemoteImportDigest::of(b"source"),
                    locator_binding_digest: RemoteImportDigest::of(b"remote-locator"),
                    baseline,
                },
                || {
                    barrier.wait();
                },
            )
        }));
    }
    for worker in workers {
        let record = worker.join().expect("refresh worker")?;
        assert_eq!(record.candidate.expect("candidate").revision.get(), 2);
    }
    assert_eq!(
        runtime
            .session(session_id)?
            .candidate
            .expect("candidate")
            .revision
            .get(),
        2
    );
    Ok(())
}

#[test]
fn concurrent_refreshes_with_different_baselines_preserve_ready_winner() -> anyhow::Result<()> {
    let fixture = Fixture::new()?;
    let mut capture = fixture.runtime.begin_prepare(fixture.request())?;
    capture.capture_file("note.md", Cursor::new(b"alpha"))?;
    let session_id = capture.session_id();
    capture.finish()?;

    let runtime = Arc::new(RemoteImportRuntime::open(&fixture.repo, fixture.repo_id)?);
    let barrier = Arc::new(Barrier::new(2));
    let baselines = [
        RemoteImportBaseline {
            ledger_head: 21.into(),
            ignore_digest: RemoteImportDigest::of(b"ignore-a"),
            locator_digest: RemoteImportDigest::of(b"local-locator-a"),
            existing: BTreeMap::new(),
        },
        RemoteImportBaseline {
            ledger_head: 22.into(),
            ignore_digest: RemoteImportDigest::of(b"ignore-b"),
            locator_digest: RemoteImportDigest::of(b"local-locator-b"),
            existing: BTreeMap::from([(
                "note.md".to_string(),
                RemoteImportDigest::of(b"different-local-content"),
            )]),
        },
    ];
    let mut workers = Vec::new();
    for baseline in baselines {
        let runtime = runtime.clone();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            let result = runtime.refresh_with_after_read_test(
                session_id,
                RemoteImportRefreshRequest {
                    expected_revision: RemoteImportCandidateRevision::FIRST,
                    source_binding_digest: RemoteImportDigest::of(b"source"),
                    locator_binding_digest: RemoteImportDigest::of(b"remote-locator"),
                    baseline: baseline.clone(),
                },
                || {
                    barrier.wait();
                },
            );
            (baseline, result)
        }));
    }

    let mut winner = None;
    let mut loser_baseline = None;
    for worker in workers {
        let (baseline, result) = worker.join().expect("refresh worker");
        match result {
            Ok(record) => winner = Some(record),
            Err(RemoteImportError::CandidateRevisionConflict { revision }) => {
                assert_eq!(revision.get(), 2);
                loser_baseline = Some(baseline);
            }
            Err(error) => return Err(error.into()),
        }
    }
    let winner = winner.expect("one refresh must publish revision 2");
    let winner_candidate = winner.candidate.as_ref().expect("winner candidate");
    assert_eq!(winner.state, RemoteImportState::Ready);
    assert_eq!(winner_candidate.revision.get(), 2);
    assert_eq!(runtime.session(session_id)?, winner);

    let retried = runtime.refresh_from_sealed(
        session_id,
        fixture.refresh_request(
            RemoteImportCandidateRevision::from_u64_for_test(2),
            loser_baseline.expect("one refresh must conflict"),
        ),
    )?;
    assert_eq!(retried.state, RemoteImportState::Ready);
    assert_eq!(
        retried.candidate.expect("retry candidate").revision.get(),
        3
    );
    Ok(())
}
