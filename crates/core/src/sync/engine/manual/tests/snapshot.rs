use super::*;

#[test]
fn manual_snapshot_rejects_envelope_seq_mismatch() -> anyhow::Result<()> {
    let (_dir, _repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    let mut response = encrypted_snapshot_with_waterline(&peer, repo_id, &key, 1)?;
    response.ops[0].peer_seq = 2.into();

    let err = engine
        .receive_remote_snapshot(response)
        .expect_err("snapshot envelope mismatch must fail closed");
    assert!(err.to_string().contains("seq mismatch"));
    assert_eq!(engine.pending_ops_count(), 0);
    Ok(())
}

#[test]
fn manual_receive_buffers_remote_snapshot_until_confirmed() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    let response = encrypted_snapshot_with_waterline(&peer, repo_id, &key, 1)?;

    assert_eq!(engine.receive_remote_snapshot(response)?, 1);
    assert_eq!(engine.pending_ops_count(), 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);

    assert_eq!(engine.merge_pending()?, 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}

// NET-017: apply-side monotonicity and contiguity for inbound remote facts.
// The stale-snapshot tests here plus the seq-gap, replay-idempotency and
// snapshot-base tests in the parent module are the automated coverage for the case.
#[test]
fn auto_stale_snapshot_does_not_regress_peer_vector() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Auto)?;
    let peer = PeerId::new("remote");
    // 用连续增量 op 把该 peer 的 vector 推进到 2。
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 2)?)?;
    assert_eq!(engine.version_vector().get(&peer), 2);
    let shadow_before = repo.get_shadow_max_seq(&peer, &repo_id)?;

    // 陈旧快照 (max seq 1 < 当前 2) 不得 reset 影子库、也不得回退 peer 的 vector。
    // 见 plan 07_network#server-ws-runtime「不得破坏 vector monotonicity」。
    engine.apply_remote_snapshot(encrypted_snapshot_with_waterline(&peer, repo_id, &key, 1)?)?;

    assert_eq!(
        engine.version_vector().get(&peer),
        2,
        "stale snapshot regressed peer vector"
    );
    assert_eq!(
        repo.get_shadow_max_seq(&peer, &repo_id)?,
        shadow_before,
        "stale snapshot wiped newer shadow ops"
    );
    Ok(())
}

#[test]
fn persisted_shadow_waterline_blocks_stale_snapshot_from_another_engine() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut newer_engine) = build_engine(SyncMode::Auto)?;
    let peer = PeerId::new("remote");
    let mut stale_engine = SyncEngine::new(
        repo.local_peer_id().clone(),
        repo.clone(),
        SyncMode::Auto,
        Some(key.clone()),
    );
    newer_engine
        .apply_remote_snapshot(encrypted_snapshot_with_waterline(&peer, repo_id, &key, 2)?)?;
    assert_eq!(stale_engine.version_vector().get(&peer), 0);

    stale_engine
        .apply_remote_snapshot(encrypted_snapshot_with_waterline(&peer, repo_id, &key, 1)?)?;

    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 2);
    assert_eq!(stale_engine.version_vector().get(&peer), 2);
    Ok(())
}

#[test]
fn auto_newer_snapshot_cannot_rewrite_confirmed_prefix() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Auto)?;
    let peer = PeerId::new("remote");
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
    let mut conflicting = encrypted_snapshot_with_waterline(&peer, repo_id, &key, 2)?;
    let replacement = LedgerEntry::new_content(
        DocId::from_u128(1),
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
    conflicting.ops[0] = key.encrypt(&replacement, 1_u64.into())?;

    let error = engine
        .apply_remote_snapshot(conflicting)
        .expect_err("new snapshot must preserve the confirmed prefix");
    assert!(error.to_string().contains("sequence_conflict"));
    assert_eq!(engine.version_vector().get(&peer), 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}

#[test]
fn manual_equal_waterline_snapshots_must_be_identical() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    let first = encrypted_snapshot_with_waterline(&peer, repo_id, &key, 1)?;
    let mut conflicting = encrypted_snapshot_with_waterline(&peer, repo_id, &key, 1)?;
    let replacement = LedgerEntry::new_content(
        DocId::from_u128(1),
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
    conflicting.ops[0] = key.encrypt(&replacement, 1_u64.into())?;
    engine.buffer_remote_snapshot(first)?;
    engine.buffer_remote_snapshot(conflicting)?;

    let error = engine
        .merge_pending()
        .expect_err("same-waterline snapshots must not disagree");
    assert!(error.to_string().contains("sequence_conflict"));
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);
    assert_eq!(engine.pending_ops_count(), 2);
    Ok(())
}

#[test]
fn manual_snapshot_base_rejects_conflicting_incremental_prefix() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.buffer_remote_snapshot(encrypted_snapshot_with_waterline(&peer, repo_id, &key, 1)?)?;
    engine.buffer_remote_ops(conflicting_prefix_response(&peer, repo_id, &key)?)?;

    let error = engine
        .merge_pending()
        .expect_err("incremental prefix must equal the selected snapshot base");
    assert!(error.to_string().contains("sequence_conflict"));
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);
    assert_eq!(engine.pending_ops_count(), 3);
    Ok(())
}

#[test]
fn manual_stale_snapshot_does_not_regress_or_wipe_newer_ops() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 2)?)?;
    assert_eq!(engine.version_vector().get(&peer), 2);

    // 缓冲一个陈旧快照后 merge：它被 newer 状态 supersede，应被丢弃而非应用。
    engine.buffer_remote_snapshot(encrypted_snapshot_with_waterline(&peer, repo_id, &key, 1)?)?;
    engine.merge_pending()?;

    assert_eq!(
        engine.version_vector().get(&peer),
        2,
        "stale snapshot regressed peer vector"
    );
    assert_eq!(
        repo.get_shadow_max_seq(&peer, &repo_id)?,
        2,
        "stale snapshot wiped newer shadow ops"
    );
    assert_eq!(engine.pending_ops_count(), 0);
    Ok(())
}

#[test]
fn manual_snapshot_base_allows_newer_contiguous_ops() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");

    engine.buffer_remote_snapshot(encrypted_snapshot_with_waterline(&peer, repo_id, &key, 3)?)?;
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 4)?)?;

    assert_eq!(engine.merge_pending()?, 4);
    assert_eq!(
        engine.version_vector().get(&peer),
        4,
        "snapshot base plus newer op did not advance vector"
    );
    assert_eq!(
        repo.get_shadow_ops_in_range(&peer, &repo_id, 1.into(), 4.into())?
            .len(),
        4,
        "snapshot base and newer op should apply in one shadow transaction"
    );
    assert_eq!(engine.pending_ops_count(), 0);
    Ok(())
}

#[test]
fn failed_manual_snapshot_merge_does_not_reset_shadow_repo() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.apply_remote_ops(encrypted_response(&peer, repo_id, &key)?)?;
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);

    engine.buffer_remote_snapshot(tampered_response(&peer, repo_id))?;

    let err = engine.merge_pending().expect_err("bad snapshot must fail");
    assert!(err.to_string().contains("Decryption failed"));
    assert_eq!(engine.pending_ops_count(), 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}

#[test]
fn failed_manual_snapshot_validation_does_not_reset_shadow_repo() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.apply_remote_ops(encrypted_response(&peer, repo_id, &key)?)?;
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);

    engine.buffer_remote_snapshot(encrypted_invalid_snapshot(&peer, repo_id, &key)?)?;

    let err = engine
        .merge_pending()
        .expect_err("bad snapshot must fail ledger validation");
    assert!(err.to_string().contains("Refusing to append content op"));
    assert_eq!(engine.pending_ops_count(), 2);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}

fn encrypted_invalid_snapshot(
    peer: &PeerId,
    repo_id: RepoId,
    key: &RepoKey,
) -> anyhow::Result<SyncResponse> {
    let doc_id = DocId::from_u128(1);
    let first = LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos: 0,
            content: "x".into(),
        },
        1,
        peer.clone(),
        1,
        None,
        None,
    );
    let invalid = LedgerEntry::new_content(
        doc_id,
        Op::Delete { pos: 99, len: 1 },
        2,
        peer.clone(),
        2,
        None,
        None,
    );
    Ok(SyncResponse::full_fact_replay(
        peer.clone(),
        repo_id,
        2_u64.into(),
        vec![
            key.encrypt(&first, 1_u64.into())?,
            key.encrypt(&invalid, 2_u64.into())?,
        ],
    ))
}
