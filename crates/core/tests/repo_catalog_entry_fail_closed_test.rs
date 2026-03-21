use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;

#[test]
fn local_repo_listing_fails_closed_on_unexpected_non_redb_entry() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = RepoManager::init(dir.path(), 8, Some("default"), Some("urn:default"))?;
    let local_dir = dir.path().join("local");
    std::fs::write(local_dir.join("stray.txt"), "oops")?;

    let err = repo
        .list_repos(None)
        .expect_err("unexpected local non-redb entry must fail closed");
    assert!(err.to_string().contains("unexpected non-redb entry"));
    Ok(())
}

#[test]
fn remote_repo_listing_fails_closed_on_unexpected_non_redb_entry() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let repo = RepoManager::init(dir.path(), 8, Some("default"), Some("urn:default"))?;
    let peer = PeerId::new("peer-a");
    repo.ensure_shadow_repo_info(
        &peer,
        &RepoInfo {
            uuid: uuid::Uuid::new_v4(),
            name: "notes".into(),
            url: Some("urn:notes".into()),
        },
    )?;
    let peer_dir = dir.path().join("remotes").join(peer.to_string());
    std::fs::write(peer_dir.join("stray.txt"), "oops")?;

    let err = repo
        .list_repos(Some(&peer))
        .expect_err("unexpected remote non-redb entry must fail closed");
    assert!(err.to_string().contains("unexpected non-redb entry"));
    Ok(())
}
