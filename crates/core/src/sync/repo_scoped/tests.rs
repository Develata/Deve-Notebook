use super::RepoScopedSyncEngine;
use crate::config::SyncMode;
use crate::ledger::RepoManager;
use crate::models::{DocId, LedgerEntry, Op, PeerId, RepoId};
use std::fs;
use std::sync::Arc;

fn build_repo() -> anyhow::Result<(tempfile::TempDir, RepoManager, RepoId)> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, Some("notes"), Some("urn:test:notes"))?;
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    let repo_id = repo.get_repo_info()?.expect("repo info").uuid;
    Ok((dir, repo, repo_id))
}

fn append_local_doc(repo: &RepoManager, peer_id: &PeerId, content: &str) -> anyhow::Result<()> {
    let doc_id = DocId::new();
    let content = content.to_string();
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        peer_id.clone(),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.clone().into(),
                },
                1,
                peer_id.clone(),
                seq,
                None,
                None,
            )
        },
    )?;
    Ok(())
}

fn remote_doc(peer_id: &PeerId, content: &str, seq: u64) -> LedgerEntry {
    LedgerEntry::new_content(
        DocId::new(),
        Op::Insert {
            pos: 0,
            content: content.to_string().into(),
        },
        1,
        peer_id.clone(),
        seq,
        None,
        None,
    )
}

#[test]
fn constructor_preserves_configured_sync_mode() -> anyhow::Result<()> {
    let (_dir, repo, _repo_id) = build_repo()?;

    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Manual);

    assert_eq!(engine.sync_mode(), SyncMode::Manual);
    Ok(())
}

#[test]
fn strict_engine_mutation_persists_in_registry() -> anyhow::Result<()> {
    let (_dir, repo, repo_id) = build_repo()?;
    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);

    engine.with_strict_engine_mut(repo_id, |engine| {
        engine.set_sync_mode(SyncMode::Manual);
    })?;

    let loaded = engine.get_or_create_strict(repo_id)?;
    assert_eq!(loaded.sync_mode(), SyncMode::Manual);
    Ok(())
}

#[test]
fn strict_engine_load_fails_closed_when_lock_is_poisoned() -> anyhow::Result<()> {
    let (_dir, repo, repo_id) = build_repo()?;
    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = engine.engines.write().expect("write lock");
        panic!("poison repo scoped sync engine");
    }));

    let err = match engine.get_or_create_strict(repo_id) {
        Ok(_) => panic!("strict engine load must fail closed after lock poison"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("lock poisoned"));
    Ok(())
}

#[test]
fn get_returns_none_when_lock_is_poisoned() -> anyhow::Result<()> {
    let (_dir, repo, repo_id) = build_repo()?;
    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = engine.engines.write().expect("write lock");
        panic!("poison repo scoped sync engine");
    }));

    assert!(engine.get(repo_id).is_none());
    let err = engine
        .loaded_repos()
        .expect_err("loaded repos must fail closed after lock poison");
    assert!(err.to_string().contains("poisoned"));
    Ok(())
}

#[test]
fn clone_fails_closed_when_lock_is_poisoned() -> anyhow::Result<()> {
    let (_dir, repo, _repo_id) = build_repo()?;
    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = engine.engines.write().expect("write lock");
        panic!("poison repo scoped sync engine");
    }));

    let cloned = engine.clone();
    let err = cloned
        .loaded_repos()
        .expect_err("cloned registry must fail closed after poison");
    assert!(err.to_string().contains("poisoned"));
    Ok(())
}

#[test]
fn clear_fails_closed_when_lock_is_poisoned() -> anyhow::Result<()> {
    let (_dir, repo, _repo_id) = build_repo()?;
    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = engine.engines.write().expect("write lock");
        panic!("poison repo scoped sync engine");
    }));

    let err = engine
        .clear()
        .expect_err("clear must fail closed after lock poison");
    assert!(err.to_string().contains("poisoned"));
    Ok(())
}

#[test]
fn strict_engine_load_fails_closed_when_repo_key_is_corrupt() -> anyhow::Result<()> {
    let (_dir, repo, repo_id) = build_repo()?;
    let key_dir = repo.local_repo_notegit_keys_root("notes")?;
    fs::create_dir_all(&key_dir)?;
    fs::write(key_dir.join("repo.key"), [1, 2, 3, 4])?;

    let engine = RepoScopedSyncEngine::new(PeerId::new("local"), Arc::new(repo), SyncMode::Auto);

    let err = match engine.get_or_create_strict(repo_id) {
        Ok(_) => panic!("strict engine load must fail closed on corrupt repo key"),
        Err(err) => err,
    };
    assert!(err.to_string().contains("Failed to load repo key"));
    assert!(engine.get(repo_id).is_none());
    Ok(())
}

#[test]
fn strict_engine_hydrates_version_vector_from_ledger_heads() -> anyhow::Result<()> {
    let (_dir, repo, repo_id) = build_repo()?;
    let local_peer = PeerId::new("local");
    let remote_peer = PeerId::new("remote");
    append_local_doc(&repo, &local_peer, "a")?;
    append_local_doc(&repo, &local_peer, "b")?;
    repo.append_remote_ops(
        &remote_peer,
        &repo_id,
        &[
            remote_doc(&remote_peer, "remote-a", 1),
            remote_doc(&remote_peer, "remote-b", 2),
        ],
    )?;

    let engine = RepoScopedSyncEngine::new(local_peer.clone(), Arc::new(repo), SyncMode::Auto);
    let loaded = engine.get_or_create_strict(repo_id)?;

    assert_eq!(loaded.version_vector().get(&local_peer), 2);
    assert_eq!(loaded.version_vector().get(&remote_peer), 2);
    Ok(())
}

#[test]
fn strict_engine_refreshes_existing_vector_from_ledger_heads() -> anyhow::Result<()> {
    let (_dir, repo, repo_id) = build_repo()?;
    let local_peer = PeerId::new("local");
    let repo = Arc::new(repo);
    let engine = RepoScopedSyncEngine::new(local_peer.clone(), repo.clone(), SyncMode::Auto);

    assert_eq!(
        engine
            .get_or_create_strict(repo_id)?
            .version_vector()
            .get(&local_peer),
        0
    );
    append_local_doc(repo.as_ref(), &local_peer, "after-load")?;
    let refreshed =
        engine.with_strict_engine(repo_id, |engine| engine.version_vector().get(&local_peer))?;

    assert_eq!(refreshed, 1);
    Ok(())
}
