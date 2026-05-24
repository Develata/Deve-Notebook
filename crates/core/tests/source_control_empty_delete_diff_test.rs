use deve_core::ledger::RepoManager;
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, repo)
}

fn workspace_file(repo: &RepoManager, path: &str) -> std::path::PathBuf {
    repo.local_repo_workspace_path("default", path)
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
