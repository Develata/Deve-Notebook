use crate::config::SyncMode;
use crate::ledger::RepoManager;
use crate::models::{DocId, LedgerEntry, Op, PeerFactSeq, PeerId, RepoId};
use crate::security::{EncryptedOp, RepoKey};
use crate::sync::engine::SyncEngine;
use crate::sync::protocol::SyncResponse;
use std::sync::Arc;

mod snapshot;

fn build_engine(
    mode: SyncMode,
) -> anyhow::Result<(
    tempfile::TempDir,
    Arc<RepoManager>,
    RepoId,
    RepoKey,
    SyncEngine,
)> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 8, Some("notes"), Some("urn:test:notes"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    let repo = Arc::new(repo);
    let key = RepoKey::generate();
    let engine = SyncEngine::new(
        repo.local_peer_id().clone(),
        repo.clone(),
        mode,
        Some(key.clone()),
    );
    Ok((dir, repo, repo_id, key, engine))
}

fn encrypted_response(
    peer: &PeerId,
    repo_id: RepoId,
    key: &RepoKey,
) -> anyhow::Result<SyncResponse> {
    encrypted_response_with_seq(peer, repo_id, key, 1)
}

fn encrypted_response_with_seq(
    peer: &PeerId,
    repo_id: RepoId,
    key: &RepoKey,
    seq: u64,
) -> anyhow::Result<SyncResponse> {
    let doc_id = DocId::from_u128(1);
    let entry = LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos: (seq - 1) as u32,
            content: "x".into(),
        },
        1,
        peer.clone(),
        seq,
        None,
        None,
    );
    let peer_seq = PeerFactSeq::new(seq);
    Ok(SyncResponse::incremental(
        peer.clone(),
        repo_id,
        (peer_seq, peer_seq),
        vec![key.encrypt(&entry, peer_seq)?],
    ))
}

fn encrypted_snapshot_with_waterline(
    peer: &PeerId,
    repo_id: RepoId,
    key: &RepoKey,
    waterline: u64,
) -> anyhow::Result<SyncResponse> {
    let doc_id = DocId::from_u128(1);
    let mut ops = Vec::new();
    for seq in 1..=waterline {
        let entry = LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: (seq - 1) as u32,
                content: "x".into(),
            },
            seq as i64,
            peer.clone(),
            seq,
            None,
            None,
        );
        ops.push(key.encrypt(&entry, seq.into())?);
    }
    Ok(SyncResponse::full_fact_replay(
        peer.clone(),
        repo_id,
        waterline.into(),
        ops,
    ))
}

fn encrypted_invalid_delete_response(
    peer: &PeerId,
    repo_id: RepoId,
    key: &RepoKey,
    seq: u64,
) -> anyhow::Result<SyncResponse> {
    let entry = LedgerEntry::new_content(
        DocId::new(),
        Op::Delete { pos: 99, len: 1 },
        1,
        peer.clone(),
        seq,
        None,
        None,
    );
    let peer_seq = PeerFactSeq::new(seq);
    Ok(SyncResponse::incremental(
        peer.clone(),
        repo_id,
        (peer_seq, peer_seq),
        vec![key.encrypt(&entry, peer_seq)?],
    ))
}

fn conflicting_prefix_response(
    peer: &PeerId,
    repo_id: RepoId,
    key: &RepoKey,
) -> anyhow::Result<SyncResponse> {
    let doc_id = DocId::from_u128(1);
    let conflicting = LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos: 0,
            content: "y".into(),
        },
        1,
        peer.clone(),
        1,
        None,
        None,
    );
    let next = LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos: 1,
            content: "x".into(),
        },
        2,
        peer.clone(),
        2,
        None,
        None,
    );
    Ok(SyncResponse::incremental(
        peer.clone(),
        repo_id,
        (1_u64.into(), 2_u64.into()),
        vec![
            key.encrypt(&conflicting, 1_u64.into())?,
            key.encrypt(&next, 2_u64.into())?,
        ],
    ))
}

fn tampered_response(peer: &PeerId, repo_id: RepoId) -> SyncResponse {
    SyncResponse::full_fact_replay(
        peer.clone(),
        repo_id,
        1.into(),
        vec![EncryptedOp {
            doc_id: None,
            peer_seq: 1.into(),
            ciphertext: vec![1, 2, 3],
            nonce: vec![0; 12],
        }],
    )
}

fn seq_mismatch_response(
    peer: &PeerId,
    repo_id: RepoId,
    key: &RepoKey,
) -> anyhow::Result<SyncResponse> {
    let mut response = encrypted_response_with_seq(peer, repo_id, key, 1)?;
    response.ops[0].peer_seq = 2.into();
    Ok(response)
}

#[test]
fn manual_receive_buffers_remote_ops_until_confirmed() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    let response = encrypted_response(&peer, repo_id, &key)?;

    assert_eq!(engine.receive_remote_ops(response)?, 1);
    assert_eq!(engine.pending_ops_count(), 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);

    assert_eq!(engine.merge_pending()?, 1);
    assert_eq!(engine.pending_ops_count(), 0);
    assert_eq!(engine.pending_ops.payload_count(), 0);
    assert_eq!(engine.pending_ops.encoded_bytes(), 0);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}

#[test]
fn transport_clone_does_not_copy_manual_pending_queue() -> anyhow::Result<()> {
    let (_dir, _repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.receive_remote_ops(encrypted_response(&peer, repo_id, &key)?)?;

    let outbound = engine.clone_for_transport();

    assert_eq!(engine.pending_ops_count(), 1);
    assert_eq!(outbound.pending_ops_count(), 0);
    assert!(Arc::ptr_eq(&engine.repo, &outbound.repo));
    assert_eq!(outbound.local_peer_id, engine.local_peer_id);
    assert_eq!(outbound.version_vector(), engine.version_vector());
    assert!(outbound.repo_key.is_some());
    Ok(())
}

#[test]
fn manual_resource_preflight_runs_before_decrypt_and_preserves_queue() -> anyhow::Result<()> {
    let (_dir, _repo, repo_id, _key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    for _ in 0..crate::protocol::MAX_SYNC_FACTS_PER_PAYLOAD {
        engine.buffer_remote_ops(SyncResponse::incremental(
            peer.clone(),
            repo_id,
            (PeerFactSeq::ONE, PeerFactSeq::ONE),
            Vec::new(),
        ))?;
    }
    let before = (
        engine.pending_ops.payload_count(),
        engine.pending_ops_count(),
        engine.pending_ops.encoded_bytes(),
    );

    let error = engine
        .receive_remote_ops(tampered_response(&peer, repo_id))
        .expect_err("resource limit must reject before decrypting the tampered payload");
    assert!(error.to_string().contains("sync_resource_limit"));
    assert!(!error.to_string().contains("Decryption failed"));
    assert_eq!(
        (
            engine.pending_ops.payload_count(),
            engine.pending_ops_count(),
            engine.pending_ops.encoded_bytes(),
        ),
        before
    );
    Ok(())
}

#[test]
fn manual_receive_validation_failure_does_not_enqueue_or_change_counters() -> anyhow::Result<()> {
    let (_dir, _repo, repo_id, _key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");

    let error = engine
        .receive_remote_snapshot(tampered_response(&peer, repo_id))
        .expect_err("invalid payload must fail validation before enqueue");
    assert!(error.to_string().contains("Decryption failed"));
    assert_eq!(
        (
            engine.pending_ops.payload_count(),
            engine.pending_ops_count(),
            engine.pending_ops.encoded_bytes(),
        ),
        (0, 0, 0)
    );
    Ok(())
}

#[test]
fn manual_merge_rejects_incremental_seq_mismatch() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.buffer_remote_ops(seq_mismatch_response(&peer, repo_id, &key)?)?;

    let err = engine
        .merge_pending()
        .expect_err("incremental seq mismatch must fail");
    assert!(err.to_string().contains("seq mismatch"));
    assert_eq!(engine.pending_ops_count(), 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);
    Ok(())
}

#[test]
fn auto_receive_applies_remote_ops_immediately() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Auto)?;
    let peer = PeerId::new("remote");
    let response = encrypted_response(&peer, repo_id, &key)?;

    assert_eq!(engine.receive_remote_ops(response)?, 1);
    assert_eq!(engine.pending_ops_count(), 0);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}

#[test]
fn auto_apply_remote_ops_rejects_seq_gap() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Auto)?;
    let peer = PeerId::new("remote");
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
    assert_eq!(engine.version_vector().get(&peer), 1);

    // 跳过 seq 2 直接给 seq 3 会让 vector 越过一个未接收的 op、静默丢失它；
    // apply 必须 fail-closed，而不是悄悄推进 vector。
    let err = engine
        .apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 3)?)
        .expect_err("seq gap must fail closed");
    assert!(
        err.to_string().contains("sequence_gap"),
        "unexpected error: {err}"
    );
    assert_eq!(
        engine.version_vector().get(&peer),
        1,
        "vector advanced past an unreceived op"
    );
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}

#[test]
fn auto_replayed_remote_ops_do_not_append_duplicate_shadow_entries() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Auto)?;
    let peer = PeerId::new("remote");
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
    assert_eq!(engine.version_vector().get(&peer), 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);

    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;

    assert_eq!(engine.version_vector().get(&peer), 1);
    assert_eq!(
        repo.get_shadow_max_seq(&peer, &repo_id)?,
        1,
        "stale replay appended a duplicate shadow entry"
    );
    Ok(())
}

#[test]
fn auto_conflicting_prefix_rejects_entire_incremental_batch() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Auto)?;
    let peer = PeerId::new("remote");
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;

    let error = engine
        .apply_remote_ops(conflicting_prefix_response(&peer, repo_id, &key)?)
        .expect_err("conflicting confirmed prefix must reject the entire batch");
    assert!(error.to_string().contains("sequence_conflict"));
    assert_eq!(engine.version_vector().get(&peer), 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    assert!(
        repo.get_shadow_ops_in_range(&peer, &repo_id, 1.into(), 1.into())?[0]
            .1
            .content_op()
            .is_some_and(|op| matches!(op, Op::Insert { content, .. } if content == "x"))
    );
    Ok(())
}

#[test]
fn manual_merge_rejects_incremental_seq_gap() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 3)?)?;

    let err = engine
        .merge_pending()
        .expect_err("seq gap must fail closed");
    assert!(
        err.to_string().contains("sequence_gap"),
        "unexpected error: {err}"
    );
    assert_eq!(engine.pending_ops_count(), 1);
    assert_eq!(engine.version_vector().get(&peer), 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}

#[test]
fn failed_manual_merge_retains_pending_ops() -> anyhow::Result<()> {
    let (_dir, _repo, repo_id, _key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.buffer_remote_ops(tampered_response(&peer, repo_id))?;

    let err = engine.merge_pending().expect_err("tampered op must fail");
    assert!(err.to_string().contains("Decryption failed"));
    assert_eq!(engine.pending_ops_count(), 1);
    Ok(())
}

#[test]
fn failed_manual_merge_does_not_partially_apply_ops() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
    engine.buffer_remote_ops(tampered_response(&peer, repo_id))?;

    let err = engine
        .merge_pending()
        .expect_err("second payload must fail");
    assert!(err.to_string().contains("Decryption failed"));
    assert_eq!(engine.pending_ops_count(), 2);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);
    Ok(())
}

#[test]
fn failed_manual_merge_validation_rolls_back_prior_payload() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
    engine.buffer_remote_ops(encrypted_invalid_delete_response(&peer, repo_id, &key, 2)?)?;

    let err = engine
        .merge_pending()
        .expect_err("second payload must fail ledger validation");
    assert!(err.to_string().contains("Refusing to append content op"));
    assert_eq!(engine.pending_ops_count(), 2);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);
    Ok(())
}

#[test]
fn failed_manual_merge_restores_payloads_and_all_resource_counters() -> anyhow::Result<()> {
    for apply_failure in [false, true] {
        let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
        let peer = PeerId::new("remote");
        engine.buffer_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
        if apply_failure {
            engine
                .buffer_remote_ops(encrypted_invalid_delete_response(&peer, repo_id, &key, 2)?)?;
        } else {
            engine.buffer_remote_ops(tampered_response(&peer, repo_id))?;
        }
        let before = (
            engine.pending_ops.payload_count(),
            engine.pending_ops_count(),
            engine.pending_ops.encoded_bytes(),
        );

        engine
            .merge_pending()
            .expect_err("decrypt or apply failure must restore the complete pending buffer");
        assert_eq!(
            (
                engine.pending_ops.payload_count(),
                engine.pending_ops_count(),
                engine.pending_ops.encoded_bytes(),
            ),
            before
        );
        assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);
    }
    Ok(())
}

#[test]
fn manual_merge_rejects_mixed_peer_targets() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer_a = PeerId::new("remote-a");
    let peer_b = PeerId::new("remote-b");
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer_a, repo_id, &key, 1)?)?;
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer_b, repo_id, &key, 1)?)?;

    let err = engine
        .merge_pending()
        .expect_err("mixed peer targets must fail closed");
    assert!(err.to_string().contains("one peer/repo target"));
    assert_eq!(engine.pending_ops_count(), 2);
    assert_eq!(repo.get_shadow_max_seq(&peer_a, &repo_id)?, 0);
    assert_eq!(repo.get_shadow_max_seq(&peer_b, &repo_id)?, 0);
    Ok(())
}

#[test]
fn manual_merge_rejects_mixed_repo_targets() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let other_repo_id = uuid::Uuid::new_v4();
    let peer = PeerId::new("remote");
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, other_repo_id, &key, 1)?)?;

    let err = engine
        .merge_pending()
        .expect_err("mixed repo targets must fail closed");
    assert!(err.to_string().contains("one peer/repo target"));
    assert_eq!(engine.pending_ops_count(), 2);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);
    assert_eq!(repo.get_shadow_max_seq(&peer, &other_repo_id)?, 0);
    Ok(())
}
