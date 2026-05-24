use crate::config::SyncMode;
use crate::ledger::RepoManager;
use crate::models::{DocId, LedgerEntry, Op, PeerId, RepoId};
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
    let engine = SyncEngine::new(PeerId::new("local"), repo.clone(), mode, Some(key.clone()));
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
    let doc_id = DocId::new();
    let entry = LedgerEntry::new_content(
        doc_id,
        Op::Insert {
            pos: 0,
            content: "remote".into(),
        },
        1,
        peer.clone(),
        seq,
        None,
        None,
    );
    Ok(SyncResponse {
        peer_id: peer.clone(),
        repo_id,
        ops: vec![key.encrypt(&entry, seq)?],
    })
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
    Ok(SyncResponse {
        peer_id: peer.clone(),
        repo_id,
        ops: vec![key.encrypt(&entry, seq)?],
    })
}

fn tampered_response(peer: &PeerId, repo_id: RepoId) -> SyncResponse {
    SyncResponse {
        peer_id: peer.clone(),
        repo_id,
        ops: vec![EncryptedOp {
            doc_id: None,
            seq: 2,
            ciphertext: vec![1, 2, 3],
            nonce: vec![0; 12],
        }],
    }
}

fn seq_mismatch_response(
    peer: &PeerId,
    repo_id: RepoId,
    key: &RepoKey,
) -> anyhow::Result<SyncResponse> {
    let mut response = encrypted_response_with_seq(peer, repo_id, key, 1)?;
    response.ops[0].seq = 2;
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
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 1);
    Ok(())
}

#[test]
fn manual_merge_rejects_incremental_seq_mismatch() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.buffer_remote_ops(seq_mismatch_response(&peer, repo_id, &key)?);

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
fn failed_manual_merge_retains_pending_ops() -> anyhow::Result<()> {
    let (_dir, _repo, repo_id, _key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.buffer_remote_ops(tampered_response(&peer, repo_id));

    let err = engine.merge_pending().expect_err("tampered op must fail");
    assert!(err.to_string().contains("Decryption failed"));
    assert_eq!(engine.pending_ops_count(), 1);
    Ok(())
}

#[test]
fn failed_manual_merge_does_not_partially_apply_ops() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer = PeerId::new("remote");
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?);
    engine.buffer_remote_ops(tampered_response(&peer, repo_id));

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
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?);
    engine.buffer_remote_ops(encrypted_invalid_delete_response(&peer, repo_id, &key, 2)?);

    let err = engine
        .merge_pending()
        .expect_err("second payload must fail ledger validation");
    assert!(err.to_string().contains("Refusing to append content op"));
    assert_eq!(engine.pending_ops_count(), 2);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);
    Ok(())
}

#[test]
fn manual_merge_rejects_mixed_peer_targets() -> anyhow::Result<()> {
    let (_dir, repo, repo_id, key, mut engine) = build_engine(SyncMode::Manual)?;
    let peer_a = PeerId::new("remote-a");
    let peer_b = PeerId::new("remote-b");
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer_a, repo_id, &key, 1)?);
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer_b, repo_id, &key, 1)?);

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
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, repo_id, &key, 1)?);
    engine.buffer_remote_ops(encrypted_response_with_seq(&peer, other_repo_id, &key, 1)?);

    let err = engine
        .merge_pending()
        .expect_err("mixed repo targets must fail closed");
    assert!(err.to_string().contains("one peer/repo target"));
    assert_eq!(engine.pending_ops_count(), 2);
    assert_eq!(repo.get_shadow_max_seq(&peer, &repo_id)?, 0);
    assert_eq!(repo.get_shadow_max_seq(&peer, &other_repo_id)?, 0);
    Ok(())
}
