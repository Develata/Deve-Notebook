use super::RepoManager;
use crate::ledger::RepoInfo;
use crate::ledger::source_control;
use crate::models::PeerId;

#[test]
fn list_loaded_shadows_fails_closed_when_registry_lock_is_poisoned() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;

    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = repo.write_shadow_dbs().expect("write lock");
        panic!("poison shadow registry");
    }));

    let err = repo
        .list_loaded_shadows()
        .expect_err("must fail closed after lock poison");
    assert!(err.to_string().contains("Shadow DB registry lock poisoned"));
    Ok(())
}

#[test]
fn ensure_shadow_repo_info_fails_closed_when_detach_registry_is_poisoned() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    let peer = PeerId::new("peer-a");
    let repo_id = uuid::Uuid::new_v4();
    repo.ensure_shadow_repo_info(
        &peer,
        &RepoInfo {
            uuid: repo_id,
            name: "old".into(),
            url: Some("urn:test:old".into()),
        },
    )?;

    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = repo.write_shadow_dbs().expect("write lock");
        panic!("poison shadow registry");
    }));

    let err = repo
        .ensure_shadow_repo_info(
            &peer,
            &RepoInfo {
                uuid: repo_id,
                name: "new".into(),
                url: Some("urn:test:new".into()),
            },
        )
        .expect_err("must fail closed after poisoned detach registry");
    assert!(err.to_string().contains("Shadow DB registry lock poisoned"));
    Ok(())
}

#[test]
fn ensure_shadow_repo_info_initializes_source_control_tables() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = RepoManager::init(dir.path(), 10, Some("default"), Some("urn:default"))?;
    let peer = PeerId::new("peer-a");
    let remote = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:wiki".into()),
    };

    repo.ensure_shadow_repo_info(&peer, &remote)?;

    let commits = repo.run_on_shadow_repo_by_id(&peer, &remote.uuid, |db| {
        source_control::list_commits(db, 10)
    })?;
    assert!(commits.is_empty());
    Ok(())
}
