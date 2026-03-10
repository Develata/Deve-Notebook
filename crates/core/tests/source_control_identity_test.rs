use deve_core::ledger::RepoManager;
use deve_core::models::DocId;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::{TempDir, tempdir};

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path(), 10, None, None).expect("init repo");
    repo.set_vault_root(dir.path().join("vault"));
    (dir, repo)
}

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn seed_pending(
    repo: &RepoManager,
    path: &str,
    doc_id: Option<DocId>,
    status: ChangeStatus,
    content: &str,
) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: None,
                doc_id,
                change_type: status,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending entry");
}

#[test]
fn diff_uses_pending_doc_identity_for_renamed_file() {
    let (dir, repo) = new_repo();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", None, ChangeStatus::Added, "hello");
    repo.stage_pending("notes/a.md").expect("stage a");
    repo.commit_staged("initial").expect("commit a");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup doc id")
        .expect("existing doc");

    write_workspace_file(&dir, "notes/b.md", "hello renamed");
    std::fs::remove_file(dir.path().join("vault").join("default").join("notes/a.md"))
        .expect("remove old path");
    seed_pending(&repo, "notes/a.md", Some(doc_id), ChangeStatus::Deleted, "");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello renamed"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed rename candidate");

    let diff = repo.diff_doc_path("notes/b.md").expect("diff renamed path");
    assert!(diff.contains("-hello"));
    assert!(diff.contains("+hello renamed"));
    let pending = repo.list_pending_fs().expect("list pending");
    assert_eq!(
        pending
            .iter()
            .find(|entry| entry.path == "notes/b.md")
            .and_then(|entry| entry.renamed_from.as_deref()),
        Some("notes/a.md")
    );
}

#[test]
fn commit_preserves_doc_id_for_rename_candidate() {
    let (dir, repo) = new_repo();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", None, ChangeStatus::Added, "hello");
    repo.stage_pending("notes/a.md").expect("stage a");
    repo.commit_staged("initial").expect("commit a");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup doc id")
        .expect("existing doc");

    write_workspace_file(&dir, "notes/b.md", "hello");
    std::fs::remove_file(dir.path().join("vault").join("default").join("notes/a.md"))
        .expect("remove old path");
    seed_pending(&repo, "notes/a.md", Some(doc_id), ChangeStatus::Deleted, "");
    seed_pending(
        &repo,
        "notes/b.md",
        Some(doc_id),
        ChangeStatus::Added,
        "hello",
    );
    repo.stage_pending("notes/a.md").expect("stage delete");
    repo.stage_pending("notes/b.md").expect("stage add");
    let commit = repo.commit_staged("rename").expect("commit rename");

    assert_eq!(
        repo.get_docid("notes/b.md").expect("new path"),
        Some(doc_id)
    );
    assert!(repo.get_docid("notes/a.md").expect("old path").is_none());
    assert_eq!(commit.doc_count, 1);
}
