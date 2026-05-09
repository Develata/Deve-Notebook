use super::source_control_test_support::ProxyHarness;
use deve_core::ledger::RepoManager;
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeStatus, SourceControlApi};
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::TempDir;

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault").join("default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn seed_pending(repo: &RepoManager, entry: PendingFsEntry) {
    repo.run_on_local_repo(repo.local_repo_name(), |db| pending_fs::upsert(db, &entry))
        .expect("seed pending entry");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_proxy_commit_diff_reports_rename() -> anyhow::Result<()> {
    let harness = ProxyHarness::spawn().await?;
    let dir = &harness.dir;
    let repo = harness.repo.clone();
    let proxy = harness.proxy.clone();
    let selector = RepoSelector::default();
    write_workspace_file(dir, "notes/a.md", "hello");
    seed_pending(
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
    proxy.stage_pending_in_repo(&selector, &ScPathTarget::from_path("notes/a.md"))?;
    let first = proxy.commit_staged_in_repo(&selector, "initial")?;
    let doc_id = repo.get_docid("notes/a.md")?.expect("existing doc id");

    write_workspace_file(dir, "notes/b.md", "hello");
    std::fs::remove_file(dir.path().join("vault/default/notes/a.md"))?;
    seed_pending(
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
    seed_pending(
        &repo,
        PendingFsEntry {
            path: "notes/b.md".into(),
            renamed_from: Some("notes/a.md".into()),
            doc_id: Some(doc_id),
            change_type: ChangeStatus::Added,
            content_hash: pending_fs::content_hash("hello"),
            detected_at: 2,
            has_conflict: false,
        },
    );
    proxy.stage_pending_in_repo(
        &selector,
        &ScPathTarget {
            path: "notes/b.md".into(),
            doc_id: Some(doc_id),
        },
    )?;
    let second = proxy.commit_staged_in_repo(&selector, "rename")?;

    let diffs = proxy.diff_commits_in_repo(&selector, Some(&first.id), &second.id)?;
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].doc_id, Some(doc_id));
    assert_eq!(diffs[0].status, ChangeStatus::Renamed);
    assert_eq!(diffs[0].previous_path.as_deref(), Some("notes/a.md"));
    assert_eq!(diffs[0].path, "notes/b.md");
    harness.shutdown().await;
    Ok(())
}
