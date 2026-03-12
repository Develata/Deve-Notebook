use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::sync::SyncManager;
use tempfile::TempDir;

fn new_repo() -> (TempDir, std::sync::Arc<RepoManager>) {
    let dir = TempDir::new().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, std::sync::Arc::new(repo))
}

fn seed_file(repo: &RepoManager, doc_path: &str, content: &str) {
    let doc_id = repo
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
fn rebuild_projection_force_overwrites_and_prunes_stale_markdown() {
    let (dir, repo) = new_repo();
    repo.apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes/empty", "test")
        .expect("create dir");
    seed_file(&repo, "notes/a.md", "ledger");
    let root = dir.path().join("vault").join("default");
    std::fs::create_dir_all(root.join("notes/ghost")).expect("mkdirs");
    std::fs::write(root.join("notes/a.md"), "dirty").expect("write dirty");
    std::fs::write(root.join("notes/ghost/old.md"), "stale").expect("write stale");
    std::fs::write(root.join("notes/ghost/keep.bin"), "keep").expect("write attachment");
    std::fs::create_dir_all(root.join(".notegit")).expect("mkdir .notegit");
    std::fs::write(root.join(".notegit/state.json"), "{}").expect("write state");

    let sync = SyncManager::new(repo, dir.path().join("vault"));
    sync.rebuild_projection_local_repo("default")
        .expect("rebuild");

    assert_eq!(
        std::fs::read_to_string(root.join("notes/a.md")).expect("read doc"),
        "ledger"
    );
    assert!(!root.join("notes/ghost/old.md").exists());
    assert!(root.join("notes/ghost/keep.bin").exists());
    assert!(root.join(".notegit/state.json").exists());
    assert!(root.join("notes/empty").is_dir());
}

#[test]
fn rebuild_projection_prefers_node_path_over_legacy_doc_mapping() {
    let (dir, repo) = new_repo();
    seed_file(repo.as_ref(), "notes/a.md", "ledger");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("doc id");
    inject_legacy_doc_path(repo.as_ref(), doc_id, "stale/a.md");

    let root = dir.path().join("vault").join("default");
    std::fs::create_dir_all(root.join("stale")).expect("mkdir stale");
    std::fs::write(root.join("stale/a.md"), "legacy").expect("write stale");

    let sync = SyncManager::new(repo, dir.path().join("vault"));
    sync.rebuild_projection_local_repo("default")
        .expect("rebuild");

    assert_eq!(
        std::fs::read_to_string(root.join("notes/a.md")).expect("read canonical doc"),
        "ledger"
    );
    assert!(!root.join("stale/a.md").exists());
}

#[test]
fn rebuild_projection_ignores_metadata_only_legacy_mapping() {
    let (dir, repo) = new_repo();
    let doc_id = deve_core::models::DocId::new();
    repo.append_generated_op_in_local_repo(
        repo.local_repo_name(),
        doc_id,
        PeerId::new("local"),
        |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 0,
                    content: "legacy".into(),
                },
                1,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        },
    )
    .expect("append legacy content");
    inject_legacy_doc_path(repo.as_ref(), doc_id, "legacy/orphan.md");

    let root = dir.path().join("vault").join("default");
    std::fs::create_dir_all(root.join("legacy")).expect("mkdir legacy");
    std::fs::write(root.join("legacy/orphan.md"), "stale").expect("write stale");

    let sync = SyncManager::new(repo, dir.path().join("vault"));
    sync.rebuild_projection_local_repo("default")
        .expect("rebuild");

    assert!(!root.join("legacy/orphan.md").exists());
}
