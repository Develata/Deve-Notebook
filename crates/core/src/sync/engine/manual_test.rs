use crate::config::SyncMode;
use crate::ledger::RepoManager;
use crate::models::{DocId, LedgerEntry, Op, PeerId, RepoId};
use crate::security::{EncryptedOp, RepoKey};
use crate::sync::engine::SyncEngine;
use crate::sync::protocol::SyncResponse;
use std::sync::Arc;

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
    let vault = dir.path().join("vault");
    let mut repo = RepoManager::init(dir.path(), 8, Some("notes"), Some("urn:test:notes"))?;
    repo.set_vault_root(&vault);
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
