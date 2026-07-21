use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::PeerId;

mod common;

#[test]
fn local_repo_listing_ignores_unexpected_nonmember_and_repair_reports_it() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let (repo, repo_id) =
        common::init_cataloged_repo_with_depth(&ledger_dir, &dir.path().join("notes"), 8)?;
    let local_dir = ledger_dir.join("local");
    std::fs::write(local_dir.join("stray.txt"), "oops")?;

    assert_eq!(repo.list_repos(None)?, vec![repo_id.to_string()]);
    let err = repo
        .repair_local_repo_catalog()
        .expect_err("explicit repair must report unexpected local non-redb entry");
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
