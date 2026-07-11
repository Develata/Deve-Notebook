use deve_core::ledger::merge::{MergeEvaluation, MergeResult};
use deve_core::ledger::schema::MERGE_BASE_CHECKPOINT;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{DocId, FactActor, LedgerEntry, MergeResolution, Op, PeerId};
use tempfile::tempdir;

struct Fixture {
    _dir: tempfile::TempDir,
    repo: RepoManager,
    repo_id: uuid::Uuid,
    peer: PeerId,
    doc_id: DocId,
}

impl Fixture {
    fn new(local: &str, remote: &str) -> anyhow::Result<Self> {
        let dir = tempdir()?;
        let repo = RepoManager::init(dir.path(), 10, Some("default"), None)?;
        let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
        let peer = PeerId::new("remote-physical-peer");
        repo.ensure_shadow_repo_info(
            &peer,
            &RepoInfo {
                uuid: repo_id,
                name: "default".into(),
                url: None,
            },
        )?;
        let doc_id = DocId::new();
        append_local(
            &repo,
            doc_id,
            Op::Insert {
                pos: 0,
                content: local.into(),
            },
        )?;
        repo.append_remote_op(
            &peer,
            &repo_id,
            &LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: remote.into(),
                },
                1,
                peer.clone(),
                1,
                None,
                None,
            ),
        )?;
        Ok(Self {
            _dir: dir,
            repo,
            repo_id,
            peer,
            doc_id,
        })
    }

    fn evaluate(&self) -> anyhow::Result<MergeEvaluation> {
        self.repo.merge_peer_in_local_repo(
            self.repo.local_repo_name(),
            &self.peer,
            &self.repo_id,
            self.doc_id,
        )
    }

    fn establish_equal(&self) -> anyhow::Result<()> {
        let evaluation = self.evaluate()?;
        assert!(evaluation.preflight.establishes_equal_baseline());
        let MergeResult::Success(content) = evaluation.result else {
            panic!("equal first merge must be conflict-free");
        };
        self.repo.commit_peer_merge_in_local_repo(
            self.repo.local_repo_name(),
            &evaluation.preflight,
            &content,
            MergeResolution::EstablishEqual,
        )?;
        Ok(())
    }
}

#[test]
fn first_divergent_merge_requires_an_explicit_base() -> anyhow::Result<()> {
    let fixture = Fixture::new("local", "remote")?;
    let error = fixture
        .evaluate()
        .expect_err("divergent histories have no proven base");
    assert!(error.to_string().contains("merge_base_missing"));
    assert!(
        fixture
            .repo
            .get_merge_base_checkpoint_in_local_repo(
                fixture.repo.local_repo_name(),
                &fixture.peer,
                fixture.doc_id,
            )?
            .is_none()
    );
    Ok(())
}

#[test]
fn merge_rejects_missing_target_doc_on_either_side() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let repo = RepoManager::init(dir.path(), 10, Some("default"), None)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let peer = PeerId::new("remote-physical-peer");
    repo.ensure_shadow_repo_info(
        &peer,
        &RepoInfo {
            uuid: repo_id,
            name: "default".into(),
            url: None,
        },
    )?;
    let target = DocId::new();
    let other = DocId::new();
    append_local(
        &repo,
        target,
        Op::Insert {
            pos: 0,
            content: "target".into(),
        },
    )?;
    repo.append_remote_op(
        &peer,
        &repo_id,
        &LedgerEntry::new_content(
            other,
            Op::Insert {
                pos: 0,
                content: "other".into(),
            },
            1,
            peer.clone(),
            1,
            None,
            None,
        ),
    )?;
    let error = repo
        .merge_peer_in_local_repo("default", &peer, &repo_id, target)
        .expect_err("source target doc must exist");
    assert!(error.to_string().contains("merge_source_doc_missing"));

    let missing_local = DocId::new();
    repo.append_remote_op(
        &peer,
        &repo_id,
        &LedgerEntry::new_content(
            missing_local,
            Op::Insert {
                pos: 0,
                content: "remote-only".into(),
            },
            2,
            peer.clone(),
            2,
            None,
            None,
        ),
    )?;
    let error = repo
        .merge_peer_in_local_repo("default", &peer, &repo_id, missing_local)
        .expect_err("local target doc must exist");
    assert!(error.to_string().contains("merge_local_doc_missing"));
    Ok(())
}

#[test]
fn equal_baseline_and_accept_current_both_append_anchors() -> anyhow::Result<()> {
    let fixture = Fixture::new("base", "base")?;
    fixture.establish_equal()?;
    let first = fixture
        .repo
        .get_merge_base_checkpoint_in_local_repo(
            fixture.repo.local_repo_name(),
            &fixture.peer,
            fixture.doc_id,
        )?
        .expect("initial checkpoint");
    assert_eq!(first.source_peer_seq, 1_u64);
    assert_eq!(first.local_anchor_peer_seq, 2_u64);

    replace_local(&fixture.repo, fixture.doc_id, "base", "local")?;
    replace_remote(
        &fixture.repo,
        &fixture.peer,
        &fixture.repo_id,
        fixture.doc_id,
        "base",
        "remote",
        2,
    )?;
    let evaluation = fixture.evaluate()?;
    assert!(!evaluation.preflight.establishes_equal_baseline());
    assert!(matches!(evaluation.result, MergeResult::Conflict { .. }));
    let outcome = fixture.repo.commit_peer_merge_in_local_repo(
        fixture.repo.local_repo_name(),
        &evaluation.preflight,
        "local",
        MergeResolution::AcceptCurrent,
    )?;
    assert!(!outcome.content_changed);
    assert_eq!(outcome.anchor_peer_seq, 5_u64);

    let second = fixture
        .repo
        .get_merge_base_checkpoint_in_local_repo(
            fixture.repo.local_repo_name(),
            &fixture.peer,
            fixture.doc_id,
        )?
        .expect("updated checkpoint");
    assert_eq!(second.source_peer_seq, 3_u64);
    assert_eq!(second.local_anchor_peer_seq, outcome.anchor_peer_seq);
    assert_ne!(second.anchor_global_seq, first.anchor_global_seq);
    Ok(())
}

#[test]
fn source_drift_rejects_entire_merge_commit() -> anyhow::Result<()> {
    let fixture = Fixture::new("base", "base")?;
    fixture.establish_equal()?;
    replace_remote(
        &fixture.repo,
        &fixture.peer,
        &fixture.repo_id,
        fixture.doc_id,
        "base",
        "remote",
        2,
    )?;
    let evaluation = fixture.evaluate()?;
    let before = fixture.repo.get_local_peer_waterline(&fixture.repo_id)?;
    let other_doc = DocId::new();
    fixture.repo.append_remote_op(
        &fixture.peer,
        &fixture.repo_id,
        &LedgerEntry::new_content(
            other_doc,
            Op::Insert {
                pos: 0,
                content: "unrelated".into(),
            },
            4,
            fixture.peer.clone(),
            4,
            None,
            None,
        ),
    )?;
    let error = fixture
        .repo
        .commit_peer_merge_in_local_repo(
            fixture.repo.local_repo_name(),
            &evaluation.preflight,
            "remote",
            MergeResolution::Auto,
        )
        .expect_err("source waterline drift must fail closed");
    assert!(
        error
            .to_string()
            .contains("merge_preflight_invalid_or_stale")
    );
    assert_eq!(
        fixture.repo.get_local_peer_waterline(&fixture.repo_id)?,
        before
    );
    Ok(())
}

#[test]
fn checkpoint_survives_reopen_and_anchor_is_in_peer_range() -> anyhow::Result<()> {
    let fixture = Fixture::new("base", "base")?;
    fixture.establish_equal()?;
    let local_peer = fixture.repo.local_peer_id().clone();
    let anchor_range = fixture.repo.get_local_ops_in_range(
        &fixture.repo_id,
        &local_peer,
        2_u64.into(),
        2_u64.into(),
    )?;
    assert_eq!(anchor_range.len(), 1);
    assert!(anchor_range[0].1.merge_anchor().is_some());

    let Fixture {
        _dir: dir,
        repo,
        repo_id,
        peer,
        doc_id,
    } = fixture;
    drop(repo);
    let reopened = RepoManager::init(dir.path(), 10, Some("default"), None)?;
    let checkpoint = reopened
        .get_merge_base_checkpoint_in_local_repo("default", &peer, doc_id)?
        .expect("checkpoint after reopen");
    assert_eq!(checkpoint.source_peer_seq, 1_u64);
    let evaluation = reopened.merge_peer_in_local_repo("default", &peer, &repo_id, doc_id)?;
    assert!(!evaluation.preflight.establishes_equal_baseline());
    assert!(matches!(evaluation.result, MergeResult::Success(ref value) if value == "base"));
    Ok(())
}

#[test]
fn dangling_checkpoint_anchor_fails_closed() -> anyhow::Result<()> {
    let fixture = Fixture::new("base", "base")?;
    fixture.establish_equal()?;
    let mut checkpoint = fixture
        .repo
        .get_merge_base_checkpoint_in_local_repo(
            fixture.repo.local_repo_name(),
            &fixture.peer,
            fixture.doc_id,
        )?
        .expect("checkpoint");
    checkpoint.anchor_global_seq = checkpoint.anchor_global_seq.saturating_add(10_000);
    fixture
        .repo
        .run_on_local_repo(fixture.repo.local_repo_name(), |db| {
            let write = db.begin_write()?;
            let bytes = deve_core::codec::encode(&checkpoint)?;
            write.open_table(MERGE_BASE_CHECKPOINT)?.insert(
                (fixture.peer.as_str(), fixture.doc_id.as_u128()),
                bytes.as_slice(),
            )?;
            write.commit()?;
            Ok(())
        })?;
    let error = fixture
        .evaluate()
        .expect_err("dangling anchor reference must fail closed");
    assert!(
        error
            .to_string()
            .contains("merge_checkpoint_anchor_index_mismatch")
    );
    Ok(())
}

#[test]
fn auto_incoming_and_both_resolutions_append_typed_anchors() -> anyhow::Result<()> {
    assert_resolution_anchor(MergeResolution::Auto, "remote", false)?;
    assert_resolution_anchor(MergeResolution::AcceptIncoming, "remote", true)?;
    assert_resolution_anchor(MergeResolution::AcceptBoth, "combined", true)?;
    Ok(())
}

fn assert_resolution_anchor(
    resolution: MergeResolution,
    target: &str,
    diverge_local: bool,
) -> anyhow::Result<()> {
    let fixture = Fixture::new("base", "base")?;
    fixture.establish_equal()?;
    if diverge_local {
        replace_local(&fixture.repo, fixture.doc_id, "base", "local")?;
    }
    replace_remote(
        &fixture.repo,
        &fixture.peer,
        &fixture.repo_id,
        fixture.doc_id,
        "base",
        "remote",
        2,
    )?;
    let evaluation = fixture.evaluate()?;
    let outcome = fixture.repo.commit_peer_merge_in_local_repo(
        fixture.repo.local_repo_name(),
        &evaluation.preflight,
        target,
        resolution,
    )?;
    let entries = fixture
        .repo
        .get_local_ops_in_local_repo(fixture.repo.local_repo_name(), fixture.doc_id)?;
    let anchor = entries
        .iter()
        .find_map(|(global_seq, entry)| {
            (*global_seq == outcome.anchor_global_seq)
                .then(|| entry.merge_anchor())
                .flatten()
        })
        .expect("typed merge anchor");
    assert_eq!(anchor.resolution, resolution);
    assert_eq!(
        anchor.result_hash,
        deve_core::security::hashing::sha256_bytes(target.as_bytes())
    );
    Ok(())
}

fn append_local(repo: &RepoManager, doc_id: DocId, op: Op) -> anyhow::Result<()> {
    repo.local_fact_writer(FactActor::new("test")?)
        .append_content_in_local_repo(repo.local_repo_name(), doc_id, op, 1)?;
    Ok(())
}

fn replace_local(repo: &RepoManager, doc_id: DocId, old: &str, new: &str) -> anyhow::Result<()> {
    append_local(
        repo,
        doc_id,
        Op::Delete {
            pos: 0,
            len: old.len() as u32,
        },
    )?;
    append_local(
        repo,
        doc_id,
        Op::Insert {
            pos: 0,
            content: new.into(),
        },
    )
}

fn replace_remote(
    repo: &RepoManager,
    peer: &PeerId,
    repo_id: &uuid::Uuid,
    doc_id: DocId,
    old: &str,
    new: &str,
    first_seq: u64,
) -> anyhow::Result<()> {
    repo.append_remote_ops(
        peer,
        repo_id,
        &[
            LedgerEntry::new_content(
                doc_id,
                Op::Delete {
                    pos: 0,
                    len: old.len() as u32,
                },
                first_seq as i64,
                peer.clone(),
                first_seq,
                None,
                None,
            ),
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: new.into(),
                },
                first_seq as i64 + 1,
                peer.clone(),
                first_seq + 1,
                None,
                None,
            ),
        ],
    )?;
    Ok(())
}
