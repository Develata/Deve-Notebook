use deve_core::ledger::RepoManager;
use tempfile::{TempDir, tempdir};

mod common;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init cataloged repo");
    (dir, repo)
}

fn workspace_file(repo: &RepoManager, path: &str) -> std::path::PathBuf {
    repo.local_repo_workspace_path(repo.local_repo_name(), path)
        .expect("workspace path")
}

#[test]
fn workdir_diff_allows_deleted_empty_tracked_file() {
    let (_dir, repo) = new_repo();
    let file = workspace_file(&repo, "notes/a.md");
    repo.apply_file_structure_in_local_repo(repo.local_repo_name(), "notes/a.md", None, "test")
        .expect("create empty tracked file");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, "").expect("write empty workspace file");

    std::fs::remove_file(&file).expect("remove workspace file");

    let (old_content, new_content) = repo
        .workdir_diff_inputs_in_local_repo(repo.local_repo_name(), "notes/a.md")
        .expect("deleted empty tracked file should still diff");

    assert!(old_content.is_empty());
    assert!(new_content.is_empty());
}
