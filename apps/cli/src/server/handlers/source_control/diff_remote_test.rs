use super::remote::local_counterpart_content;
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::{TempDir, tempdir};

fn new_repo() -> anyhow::Result<(TempDir, RepoManager)> {
    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path(), 10, None, None)?;
    repo.set_vault_root(dir.path().join("vault"));
    Ok((dir, repo))
}

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn seed_pending_entry(repo: &RepoManager, entry: PendingFsEntry) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| pending_fs::upsert(db, &entry))
        .expect("seed pending entry");
}

#[test]
fn remote_diff_prefers_doc_id_for_local_counterpart() -> anyhow::Result<()> {
    let (dir, repo) = new_repo()?;
    let selector = RepoSelector::default();
    write_workspace_file(&dir, "notes/a.md", "hello");
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 1,
            has_conflict: false,
        },
    );
    repo.stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/a.md"))?;
    repo.commit_staged_in_repo(&selector, "initial")?;
    let doc_id = repo.get_docid("notes/a.md")?.expect("existing doc id");

    std::fs::remove_file(dir.path().join("vault").join("default").join("notes/a.md"))?;
    write_workspace_file(&dir, "notes/b.md", "hello renamed");
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Deleted,
            content_hash: String::new(),
            detected_at: 2,
            has_conflict: false,
        },
    );
    seed_pending_entry(
        &repo,
        PendingFsEntry {
            path: "notes/b.md".into(),
            renamed_from: Some("notes/a.md".into()),
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello renamed"),
            detected_at: 2,
            has_conflict: false,
        },
    );
    repo.stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/b.md"))?;
    repo.commit_staged_in_repo(&selector, "rename")?;

    let content = local_counterpart_content(
        &repo,
        &ScPathTarget {
            path: "notes/a.md".into(),
            doc_id: Some(doc_id),
        },
        Some(repo.local_repo_name()),
    )?;
    assert_eq!(content.as_deref(), Some("hello renamed"));
    Ok(())
}
