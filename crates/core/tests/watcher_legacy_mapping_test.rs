use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::DocId;
use deve_core::sync::SyncManager;
use deve_core::sync::scan::scan_vault;
use deve_core::vfs::Vfs;
use std::sync::Arc;
use tempfile::TempDir;

fn new_repo() -> (TempDir, Arc<RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, Arc::new(repo))
}

#[test]
fn watcher_fails_closed_when_only_legacy_path_mapping_exists() {
    let (dir, repo) = new_repo();
    let path = dir.path().join("vault/default/notes/legacy.md");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, "legacy content").expect("write file");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let doc_id = DocId::new();
        let write = db.begin_write()?;
        {
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/legacy.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/legacy.md")?;
        }
        write.commit()?;
        Ok::<_, anyhow::Error>(())
    })
    .expect("seed legacy mapping");

    let sync = SyncManager::new(repo.clone(), dir.path().join("vault"));
    let err = sync
        .handle_fs_event("default/notes/legacy.md")
        .expect_err("legacy-only watcher target must fail closed");

    assert!(
        err.to_string()
            .contains("Tracked document projection missing for legacy-mapped path")
    );
    assert!(
        repo.list_pending_fs_in_local_repo(repo.local_repo_name())
            .expect("pending list")
            .is_empty()
    );
}

#[test]
fn full_scan_fails_closed_when_only_legacy_path_mapping_exists() {
    let (dir, repo) = new_repo();
    let path = dir.path().join("vault/default/notes/legacy.md");
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, "legacy content").expect("write file");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let doc_id = DocId::new();
        let write = db.begin_write()?;
        {
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/legacy.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/legacy.md")?;
        }
        write.commit()?;
        Ok::<_, anyhow::Error>(())
    })
    .expect("seed legacy mapping");

    let vfs = Vfs::new(dir.path().join("vault"));
    let err = scan_vault(&repo, &vfs, &dir.path().join("vault"))
        .expect_err("legacy-only full scan target must fail closed");

    assert!(
        err.to_string()
            .contains("Tracked document projection missing for legacy-mapped path")
    );
    assert!(
        repo.list_pending_fs_in_local_repo(repo.local_repo_name())
            .expect("pending list")
            .is_empty()
    );
}
