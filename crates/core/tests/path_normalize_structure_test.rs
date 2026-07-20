//! STORE-010: 路径规范化 — 反斜杠路径在 structure ops 中正确转换为正斜杠。

use deve_core::ledger::RepoManager;
use deve_core::ledger::listing::RepoListing;
use deve_core::models::RepoType;
use tempfile::TempDir;

mod common;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = TempDir::new().expect("create tempdir");
    let (repo, _repo_id) = common::init_cataloged_repo_with_depth(
        &dir.path().join("ledger"),
        &dir.path().join("notes"),
        10,
    )
    .expect("init cataloged repo");
    (dir, repo)
}

#[test]
fn backslash_file_path_stored_as_forward_slash() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(&name, "folder\\sub\\a.md", None, "test")
        .expect("create file with backslash path");
    let docs = repo
        .list_docs(&RepoType::Local(
            repo.get_repo_info().unwrap().unwrap().uuid,
        ))
        .expect("list docs");
    let stored_path = docs
        .iter()
        .find(|(id, _)| *id == doc_id)
        .map(|(_, p)| p.as_str())
        .expect("doc present");
    assert_eq!(stored_path, "folder/sub/a.md");
}

#[test]
fn backslash_dir_path_stored_as_forward_slash() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let (_node_id, _ops) = repo
        .apply_dir_create_structure_in_local_repo(&name, "docs\\notes", "test")
        .expect("create dir with backslash path");
    let nodes = repo
        .list_nodes(&RepoType::Local(
            repo.get_repo_info().unwrap().unwrap().uuid,
        ))
        .expect("list nodes");
    let has_normalized = nodes.iter().any(|n| n.1.path == "docs/notes");
    assert!(has_normalized, "dir path should use forward slashes");
}

#[test]
fn mixed_slash_path_normalized_consistently() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(&name, "a\\b/c\\d.md", None, "test")
        .expect("create file with mixed slashes");
    let docs = repo
        .list_docs(&RepoType::Local(
            repo.get_repo_info().unwrap().unwrap().uuid,
        ))
        .expect("list docs");
    let stored_path = docs
        .iter()
        .find(|(id, _)| *id == doc_id)
        .map(|(_, p)| p.as_str())
        .expect("doc present");
    assert_eq!(stored_path, "a/b/c/d.md");
}

#[test]
fn backslash_delete_path_removes_forward_slash_projection() {
    let (_dir, repo) = new_repo();
    let name = repo.local_repo_name().to_string();
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(&name, "folder/sub/delete.md", None, "test")
        .expect("create file with forward slash path");

    repo.apply_file_delete_structure_in_local_repo(&name, "folder\\sub\\delete.md", None, "test")
        .expect("delete file with backslash path")
        .expect("delete op emitted");

    let docs = repo
        .list_docs(&RepoType::Local(
            repo.get_repo_info().unwrap().unwrap().uuid,
        ))
        .expect("list docs");
    assert!(
        docs.iter().all(|(id, _)| *id != doc_id),
        "delete should resolve the normalized tracked path"
    );
}
