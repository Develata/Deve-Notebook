use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::sync::SyncManager;
use tempfile::TempDir;

fn new_repo() -> (TempDir, std::sync::Arc<RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, std::sync::Arc::new(repo))
}

fn seed_file(repo: &RepoManager, doc_path: &str, content: &str) -> deve_core::models::DocId {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), doc_path, None, "test")
        .expect("create file");
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: content.into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append content");
    doc_id
}

fn inject_legacy_doc_path(repo: &RepoManager, doc_id: deve_core::models::DocId, path: &str) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        {
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            d2p.insert(doc_id.as_u128(), path)?;
            p2d.insert(path, doc_id.as_u128())?;
        }
        write.commit()?;
        Ok(())
    })
    .expect("inject legacy doc path");
}

#[test]
fn materialize_projection_creates_empty_directories_from_structure_facts() {
    let (_dir, repo) = new_repo();
    repo.apply_dir_create_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/archive/2026",
        "test",
    )
    .expect("create dir structure");

    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    sync.materialize_local_repo("default").expect("materialize");

    let root = repo
        .local_repo_workspace_root("default")
        .expect("workspace root");
    assert!(root.join("notes").is_dir());
    assert!(root.join("notes/archive").is_dir());
    assert!(root.join("notes/archive/2026").is_dir());
}

#[test]
fn materialize_projection_prefers_node_path_over_legacy_doc_mapping() {
    let (_dir, repo) = new_repo();
    seed_file(repo.as_ref(), "notes/a.md", "ledger");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");
    inject_legacy_doc_path(repo.as_ref(), doc_id, "stale/a.md");

    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    sync.materialize_local_repo("default").expect("materialize");

    let root = repo
        .local_repo_workspace_root("default")
        .expect("workspace root");
    assert_eq!(
        std::fs::read_to_string(root.join("notes/a.md")).expect("read canonical doc"),
        "ledger"
    );
    assert!(!root.join("stale/a.md").exists());
}

#[test]
fn explicit_materialize_restores_missing_bound_workspace_file() {
    let (_dir, repo) = new_repo();
    seed_file(repo.as_ref(), "notes/a.md", "ledger");
    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    sync.materialize_local_repo("default")
        .expect("initial materialize");
    let path = repo
        .local_repo_workspace_path("default", "notes/a.md")
        .expect("workspace path");
    std::fs::remove_file(&path).expect("remove materialized file");

    sync.materialize_local_repo("default")
        .expect("explicit rematerialize");

    assert_eq!(
        std::fs::read_to_string(path).expect("read rematerialized doc"),
        "ledger"
    );
}
