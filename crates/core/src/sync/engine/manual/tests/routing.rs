//! plan_ref:
//!   - 07_network#server-ws-runtime

use super::{build_engine, encrypted_response_with_seq};
use crate::config::SyncMode;
use crate::models::PeerId;

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
