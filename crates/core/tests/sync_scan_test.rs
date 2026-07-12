use deve_core::ledger::RepoManager;
use deve_core::models::{DocId, LedgerEntry, Op};
use deve_core::source_control::ChangeStatus;
use deve_core::sync::SyncManager;
use std::sync::Arc;
use tempfile::TempDir;

fn init_repo(dir: &TempDir) -> Arc<RepoManager> {
    let ledger_dir = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo =
        RepoManager::init(&ledger_dir, 10, Some("default"), Some("urn:default")).expect("init");
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("projection base");
    Arc::new(repo)
}

fn seed_tracked_doc(repo: &RepoManager, repo_name: &str, path: &str, content: &str) -> DocId {
    let (doc_id, _ops) = repo
        .apply_file_structure_in_local_repo(repo_name, path, None, "test")
        .expect("create tracked doc");
    let peer = repo.local_peer_id().clone();
    repo.append_generated_op_in_local_repo(repo_name, doc_id, peer.clone(), |seq| {
        LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: content.into(),
            },
            1,
            peer.clone(),
            seq,
            None,
            None,
        )
    })
    .expect("append tracked content");
    doc_id
}

#[cfg(unix)]
#[test]
fn scan_fails_closed_on_unreadable_repo_dir() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new().expect("create tempdir");
    let repo = init_repo(&dir);
    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    sync.scan().expect("bootstrap workspace identity");

    let unreadable = repo
        .local_repo_workspace_path("default", "private")
        .expect("workspace path");
    std::fs::create_dir_all(&unreadable).expect("create unreadable dir");
    std::fs::write(unreadable.join("hidden.md"), "# hidden").expect("write hidden doc");
    let mut perms = std::fs::metadata(&unreadable)
        .expect("metadata")
        .permissions();
    perms.set_mode(0o000);
    std::fs::set_permissions(&unreadable, perms).expect("chmod 000");

    let err = sync.scan().expect_err("scan must fail closed");
    assert!(
        err.to_string()
            .contains("Failed to walk local repo default")
    );

    let mut restore = std::fs::metadata(&unreadable)
        .expect("metadata")
        .permissions();
    restore.set_mode(0o755);
    std::fs::set_permissions(&unreadable, restore).expect("restore perms");
}

#[test]
fn scan_fails_closed_on_markdown_path_that_is_not_a_file() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = init_repo(&dir);
    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    sync.scan().expect("bootstrap workspace identity");

    std::fs::create_dir_all(
        repo.local_repo_workspace_path("default", "broken.md")
            .expect("workspace path"),
    )
    .expect("create invalid markdown directory");

    let err = sync
        .scan()
        .expect_err("non-file markdown path must fail closed");
    assert!(err.to_string().contains("markdown path is not a file"));
}

#[test]
fn scan_ignores_git_mirror_markdown_paths() {
    let dir = TempDir::new().expect("create tempdir");
    let repo = init_repo(&dir);
    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");

    let internal = repo
        .local_repo_workspace_root("default")
        .expect("workspace root")
        .join(".git/objects/x.md");
    std::fs::create_dir_all(internal.parent().expect("parent")).expect("mkdir");
    std::fs::write(&internal, "git mirror state").expect("write");

    sync.scan().expect("scan");

    assert!(
        repo.list_pending_fs_in_local_repo("default")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn scan_clears_pending_when_tracked_path_becomes_deveignored() -> anyhow::Result<()> {
    let dir = TempDir::new().expect("create tempdir");
    let repo = init_repo(&dir);
    let sync = SyncManager::new_checked(repo.clone()).expect("sync manager");
    seed_tracked_doc(repo.as_ref(), "default", "ignored/live.md", "baseline");
    sync.scan()?;

    let file = repo.local_repo_workspace_path("default", "ignored/live.md")?;
    std::fs::write(&file, "dirty")?;
    sync.scan()?;
    assert!(
        repo.list_pending_fs_in_local_repo("default")?
            .iter()
            .any(|entry| entry.path == "ignored/live.md" && entry.status == ChangeStatus::Modified)
    );

    let root = repo.local_repo_workspace_root("default")?;
    std::fs::write(root.join(".deveignore"), "ignored/*.md\n")?;
    sync.scan()?;

    assert!(
        repo.list_pending_fs_in_local_repo("default")?
            .iter()
            .all(|entry| entry.path != "ignored/live.md")
    );
    Ok(())
}
