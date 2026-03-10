use deve_core::ledger::RepoManager;
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
