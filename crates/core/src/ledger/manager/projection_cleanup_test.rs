use super::drop_transient_file_path;
use crate::ledger::RepoManager;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, repo)
}

#[test]
fn drops_legacy_projection_without_ledger_facts() {
    let (_dir, repo) = new_repo();
    repo.create_docid("notes/new.md").expect("legacy doc id");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        drop_transient_file_path(db, "notes/new.md")
    })
    .expect("drop transient projection");

    assert!(
        repo.get_docid("notes/new.md")
            .expect("lookup doc id")
            .is_none()
    );
}

#[test]
fn keeps_empty_file_projection_backed_by_structure_facts() {
    let (_dir, repo) = new_repo();
    let doc_id = repo
        .apply_file_structure_in_local_repo(repo.local_repo_name(), "notes/empty.md", None, "test")
        .expect("create empty file structure");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        drop_transient_file_path(db, "notes/empty.md")
    })
    .expect("drop transient projection");

    assert_eq!(
        repo.get_docid("notes/empty.md").expect("lookup doc id"),
        Some(doc_id)
    );
}
