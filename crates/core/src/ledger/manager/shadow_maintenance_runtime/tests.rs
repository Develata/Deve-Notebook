use super::RepoManager;
use crate::ledger::RepoInfo;
use crate::models::PeerId;
use tempfile::tempdir;

#[test]
fn delete_peer_branch_evicts_shadow_db_cache() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let repo = RepoManager::init(dir.path(), 8, Some("default"), Some("urn:default"))?;
    let peer_id = PeerId::new("peer-a");
    let first = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:first".into()),
    };
    let second = RepoInfo {
        uuid: uuid::Uuid::new_v4(),
        name: "wiki".into(),
        url: Some("urn:test:second".into()),
    };
    let first_path = repo
        .remotes_dir()
        .join(peer_id.to_filename())
        .join(format!("{}.redb", first.uuid));

    repo.ensure_shadow_repo_info(&peer_id, &first)?;
    assert!(first_path.exists());

    repo.delete_peer_branch(&peer_id)?;
    assert!(!first_path.exists());

    repo.ensure_shadow_repo_info(&peer_id, &second)?;
    let second_path = repo
        .remotes_dir()
        .join(peer_id.to_filename())
        .join(format!("{}.redb", second.uuid));
    assert!(second_path.exists());
    assert_eq!(
        repo.get_repo_info_for(Some(&peer_id), Some("wiki"))?
            .expect("recreated shadow repo")
            .uuid,
        second.uuid
    );
    Ok(())
}
