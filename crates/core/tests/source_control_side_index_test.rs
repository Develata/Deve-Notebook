use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use deve_core::source_control::{ChangeStatus, staging};
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

fn seed_pending(repo: &RepoManager, path: &str, doc_id: Option<DocId>, status: ChangeStatus) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: (status == ChangeStatus::Added && doc_id.is_some())
                    .then(|| "notes/a.md".into())
                    .filter(|_| path == "notes/b.md"),
                doc_id,
                change_type: status,
                content_hash: pending_fs::content_hash(path),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed pending entry");
}

fn seed_rename_pair(dir: &TempDir, repo: &RepoManager) -> DocId {
    write_workspace_file(dir, "notes/a.md", "hello");
    seed_pending(repo, "notes/a.md", None, ChangeStatus::Added);
    repo.stage_pending("notes/a.md").expect("stage a");
    repo.commit_staged("initial").expect("commit a");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("existing");
    write_workspace_file(dir, "notes/b.md", "hello renamed");
    std::fs::remove_file(dir.path().join("vault").join("default").join("notes/a.md"))
        .expect("remove old path");
    seed_pending(repo, "notes/a.md", Some(doc_id), ChangeStatus::Deleted);
    seed_pending(repo, "notes/b.md", Some(doc_id), ChangeStatus::Added);
    doc_id
}

#[test]
fn pending_index_lists_all_paths_for_doc() {
    let (dir, repo) = new_repo();
    let doc_id = seed_rename_pair(&dir, &repo);
    let entries = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            pending_fs::list_for_doc(db, doc_id)
        })
        .expect("list pending by doc");
    let mut paths: Vec<_> = entries.into_iter().map(|entry| entry.path).collect();
    paths.sort();
    assert_eq!(paths, vec!["notes/a.md", "notes/b.md"]);
}

#[test]
fn staged_index_shrinks_after_take() {
    let (dir, repo) = new_repo();
    let doc_id = seed_rename_pair(&dir, &repo);
    repo.stage_pending("notes/a.md").expect("stage delete");
    repo.stage_pending("notes/b.md").expect("stage add");
    let entries = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            staging::list_staged_entries_for_doc(db, doc_id)
        })
        .expect("list staged by doc");
    assert_eq!(entries.len(), 2);
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        staging::take_staged(db, "notes/a.md").map(|_| ())
    })
    .expect("take staged");
    let remaining = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            staging::list_staged_entries_for_doc(db, doc_id)
        })
        .expect("list remaining");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].0, "notes/b.md");
}

#[test]
fn target_resolution_keeps_exact_deleted_path() {
    let (dir, repo) = new_repo();
    let doc_id = seed_rename_pair(&dir, &repo);
    repo.stage_pending("notes/a.md").expect("stage delete");
    repo.stage_pending("notes/b.md").expect("stage add");
    <RepoManager as Repository>::unstage_file_in_repo(
        &repo,
        &RepoSelector::default(),
        &ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(doc_id),
        },
    )
    .expect("unstage deleted path");
    let pending = repo.list_pending_fs().expect("pending after unstage");
    assert!(
        pending
            .iter()
            .any(|entry| { entry.path == "notes/a.md" && entry.status == ChangeStatus::Deleted })
    );
    let staged = repo.list_staged().expect("staged after unstage");
    assert_eq!(staged.len(), 1);
    assert_eq!(staged[0].path, "notes/b.md");
}

#[test]
fn stage_target_uses_doc_id_when_only_rename_successor_exists() {
    let (dir, repo) = new_repo();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", None, ChangeStatus::Added);
    repo.stage_pending("notes/a.md").expect("stage a");
    repo.commit_staged("initial").expect("commit a");
    let doc_id = repo
        .get_docid("notes/a.md")
        .expect("lookup")
        .expect("existing");
    write_workspace_file(&dir, "notes/b.md", "hello renamed");
    seed_pending(&repo, "notes/b.md", Some(doc_id), ChangeStatus::Added);
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("hello renamed"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed successor");

    <RepoManager as Repository>::stage_pending_in_repo(
        &repo,
        &RepoSelector::default(),
        &ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(doc_id),
        },
    )
    .expect("stage via stale path");

    let staged = repo.list_staged().expect("list staged");
    assert_eq!(staged.len(), 1);
    assert_eq!(staged[0].path, "notes/b.md");
    assert_eq!(staged[0].doc_id, Some(doc_id));
}
