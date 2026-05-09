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
