use super::{MergeConflictFixtureOptions, run};
use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::{PeerId, RepoType};

#[test]
fn fixture_seeds_divergent_local_and_remote_content() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let repo = RepoManager::init(&ledger, 10, Some("default"), None)?;
    repo.set_projection_base_for_local_repo("default", &projection_base)?;

    run(
        &ledger,
        10,
        MergeConflictFixtureOptions {
            peer: "peer-a".into(),
            repo: None,
            path: "notes/conflict.md".into(),
            base: "base".into(),
            local: "local".into(),
            remote: "remote".into(),
        },
    )?;

    let repo = RepoManager::init(&ledger, 10, None, None)?;
    let info = repo.get_repo_info()?.expect("repo info");
    let docs = repo.list_local_docs(Some("default"))?;
    let (doc_id, path) = docs.first().expect("seeded doc");
    assert_eq!(path, "notes/conflict.md");
    assert_eq!(
        std::fs::read_to_string(projection_base.join("default/notes/conflict.md"))?,
        "local"
    );
    assert_eq!(
        repo.list_docs(&RepoType::Remote(PeerId::new("peer-a"), info.uuid))?,
        vec![(*doc_id, "notes/conflict.md".to_string())]
    );
    assert_eq!(
        content_for(&repo.get_ops(&RepoType::Local(info.uuid), *doc_id)?),
        "local"
    );
    assert_eq!(
        content_for(&repo.get_ops(&RepoType::Remote(PeerId::new("peer-a"), info.uuid), *doc_id)?),
        "remote"
    );
    Ok(())
}

fn content_for(entries: &[(u64, deve_core::models::LedgerEntry)]) -> String {
    let entries = entries
        .iter()
        .map(|(_, entry)| entry.clone())
        .filter(|entry| entry.content_op().is_some())
        .collect::<Vec<_>>();
    deve_core::state::reconstruct_content(&entries)
}
