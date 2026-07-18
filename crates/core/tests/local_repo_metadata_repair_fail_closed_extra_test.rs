use deve_core::ledger::listing::RepoListing;
use deve_core::ledger::{RepoInfo, RepoManager};
use deve_core::models::{DocId, LedgerEntry, NodeId, PeerId, StructureOp};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::staging;
use tempfile::TempDir;

mod common;

fn workspace_segment(name: &str, repo_id: uuid::Uuid) -> String {
    format!("{name}--{repo_id}")
}

fn prepare_workspace_realign_case() -> anyhow::Result<(TempDir, RepoManager, RepoInfo)> {
    let dir = TempDir::new()?;
    let ledger_dir = dir.path().join("ledger");
    let mut main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main"))?;
    let wiki_info = common::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");
    let projection_base = dir.path().join("notes");
    main.set_projection_base_for_all_local_repos_checked(&projection_base)?;
    std::fs::create_dir_all(projection_base.join("wiki"))?;
    Ok((dir, main, wiki_info))
}

fn wiki_db(repo: &RepoManager) -> anyhow::Result<std::sync::Arc<redb::Database>> {
    Ok(repo.open_database(None, "wiki")?.db)
}

fn pending_entry(path: &str) -> PendingFsEntry {
    PendingFsEntry {
        path: path.into(),
        renamed_from: None,
        doc_id: None,
        change_type: ChangeStatus::Added,
        content_hash: pending_fs::content_hash("content"),
        detected_at: 1,
        has_conflict: false,
    }
}

fn assert_metadata_unchanged(repo: &RepoManager) -> anyhow::Result<()> {
    let info = repo
        .get_repo_info_for(None, Some("wiki"))?
        .expect("wiki metadata");
    assert_eq!(info.name, "wiki");
    Ok(())
}

#[test]
fn local_repo_listing_fails_closed_on_hidden_non_redb_entry() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let repo = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let local_dir = ledger_dir.join("local");
    std::fs::write(local_dir.join(".stale"), b"local-junk").expect("hidden junk");

    let list_err = repo
        .list_repos(None)
        .expect_err("hidden non-redb local entry must fail listing");
    assert!(list_err.to_string().contains("unexpected non-redb entry"));

    let exec_err = repo
        .list_local_repo_names_for_execution()
        .expect_err("hidden non-redb local entry must fail execution listing");
    assert!(exec_err.to_string().contains("unexpected non-redb entry"));
}

#[test]
fn repair_local_repo_catalog_blocks_workspace_realign_with_pending_changes() -> anyhow::Result<()> {
    let (_dir, main, _wiki_info) = prepare_workspace_realign_case()?;
    let wiki_db = wiki_db(&main)?;
    pending_fs::upsert(wiki_db.as_ref(), &pending_entry("pending.md"))?;

    let err = main
        .repair_local_repo_catalog()
        .expect_err("pending changes must block workspace root realign");

    assert!(err.to_string().contains("pending workspace change"));
    assert_metadata_unchanged(&main)?;
    Ok(())
}

#[test]
fn repair_local_repo_catalog_blocks_workspace_realign_with_staged_changes() -> anyhow::Result<()> {
    let (_dir, main, _wiki_info) = prepare_workspace_realign_case()?;
    let wiki_db = wiki_db(&main)?;
    staging::stage_pending_entry(wiki_db.as_ref(), &pending_entry("staged.md"))?;

    let err = main
        .repair_local_repo_catalog()
        .expect_err("staged changes must block workspace root realign");

    assert!(err.to_string().contains("staged source-control change"));
    assert_metadata_unchanged(&main)?;
    Ok(())
}

#[test]
fn repair_local_repo_catalog_blocks_workspace_realign_with_dirty_workspace() -> anyhow::Result<()> {
    let (dir, main, _wiki_info) = prepare_workspace_realign_case()?;
    std::fs::write(dir.path().join("notes/wiki/orphan.md"), "orphan")?;

    let err = main
        .repair_local_repo_catalog()
        .expect_err("dirty workspace must block workspace root realign");

    assert!(
        err.to_string().contains("dirty workspace"),
        "unexpected error: {err:#}"
    );
    assert_metadata_unchanged(&main)?;
    Ok(())
}

#[test]
fn repair_local_repo_catalog_blocks_workspace_realign_with_projection_fault() -> anyhow::Result<()>
{
    let (_dir, main, _wiki_info) = prepare_workspace_realign_case()?;
    let doc_id = DocId::new();
    common::append_unvalidated_local_op(
        &main,
        "wiki",
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::from_doc_id(doc_id),
                doc_id,
                parent_id: Some(NodeId::new()),
                name: "orphan.md".into(),
            },
            1,
            PeerId::new("test"),
            1,
        ),
    );

    let err = main
        .repair_local_repo_catalog()
        .expect_err("projection fault must block workspace root realign");

    assert!(err.to_string().contains("projection fault"));
    assert_metadata_unchanged(&main)?;
    Ok(())
}

#[test]
fn repair_local_repo_catalog_fails_closed_on_workspace_root_conflict() {
    let dir = TempDir::new().expect("tempdir");
    let ledger_dir = dir.path().join("ledger");
    let mut main = RepoManager::init(&ledger_dir, 8, Some("main"), Some("urn:main")).expect("main");
    let wiki_info = common::create_initialized_local_repo(&ledger_dir, "wiki", "urn:wiki");
    let projection_base = dir.path().join("projection-base");
    std::fs::create_dir_all(projection_base.join("wiki")).expect("old root");
    std::fs::create_dir_all(projection_base.join(workspace_segment("wiki", wiki_info.uuid)))
        .expect("new root");
    main.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("mount projection base");

    let err = main
        .repair_local_repo_catalog()
        .expect_err("workspace root conflict must fail closed");
    assert!(
        err.to_string().contains("current workspace root")
            && err.to_string().contains("already exists")
    );
}
