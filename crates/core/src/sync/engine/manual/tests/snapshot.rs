use super::*;

#[test]
fn manual_snapshot_allows_envelope_seq_replay() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.buffer_remote_snapshot(seq_mismatch_response(&peer, repo_id, &key)?);

    assert_eq!(engine.merge_pending()?, 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    assert_eq!(engine.version_vector().get(&peer), 2);
    Ok(())
}

#[test]
fn manual_receive_buffers_remote_snapshot_until_confirmed() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    let response = encrypted_response(&peer, repo_id, &key)?;

    assert_eq!(engine.receive_remote_snapshot(response)?, 1);
    assert_eq!(engine.pending_ops_count(), 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);

    assert_eq!(engine.merge_pending()?, 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}

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
    engine.apply_remote_snapshot(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;

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
fn manual_stale_snapshot_does_not_regress_or_wipe_newer_ops() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?)?;
    engine.apply_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 2)?)?;
    assert_eq!(engine.version_vector().get(&peer), 2);

    // 缓冲一个陈旧快照后 merge：它被 newer 状态 supersede，应被丢弃而非应用。
    engine.buffer_remote_snapshot(encrypted_response_with_seq(&peer, repo_id, &key, 1)?);
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

    engine.buffer_remote_snapshot(encrypted_response_with_seq(&peer, repo_id, &key, 3)?);
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 4)?);

    assert_eq!(engine.merge_pending()?, 2);
    assert_eq!(
        engine.version_vector().get(&peer),
        4,
        "snapshot base plus newer op did not advance vector"
    );
    assert_eq!(
        repo.get_shadow_ops_in_range(&peer, &repo_id, 1, 3)?.len(),
        2,
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

    engine.buffer_remote_snapshot(tampered_response(&peer, repo_id));

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

    engine.buffer_remote_snapshot(encrypted_invalid_delete_response(&peer, repo_id, &key, 2)?);

    let err = engine
        .merge_pending()
        .expect_err("bad snapshot must fail ledger validation");
    assert!(err.to_string().contains("Refusing to append content op"));
    assert_eq!(engine.pending_ops_count(), 1);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}
