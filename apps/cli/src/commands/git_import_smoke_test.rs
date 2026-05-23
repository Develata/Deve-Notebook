use super::git;
use super::git_import_smoke_support::{
    assert_clean_resolved_import_export, assert_push_blocker, commit_deve_file, current_branch,
    git_cmd, git_success, init_bare_remote, init_git_repo, prepare_exported_baseline, push_report,
    resolve_imported_change_to_queued_commit, write_workspace_file,
};
use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};

#[test]
fn git_import_command_dry_run_is_read_only_and_apply_writes_pending() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let vault_root = dir.path().join("vault");
    let repo_root = vault_root.join("default");
    {
        let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
        repo.set_projection_base_for_all_local_repos(&vault_root);
        init_git_repo(&repo_root);
        commit_deve_file(&dir, &repo, "note.md", "hello\n")?;
        git_cmd(&repo_root, &["add", "."]);
        git_cmd(&repo_root, &["commit", "--no-gpg-sign", "-m", "baseline"]);
    }

    write_workspace_file(&dir, "note.md", "hello import\n");
    write_workspace_file(&dir, "new.md", "new file\n");

    git::import(&ledger_dir, Some("default"), false, 10)?;
    {
        let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
        repo.set_projection_base_for_all_local_repos(&vault_root);
        let pending = repo.list_pending_fs_in_local_repo("default")?;
        assert!(pending.is_empty(), "{pending:?}");
    }

    git::import(&ledger_dir, Some("default"), true, 10)?;
    let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
    repo.set_projection_base_for_all_local_repos(&vault_root);
    let pending = repo.list_pending_fs_in_local_repo("default")?;
    assert_eq!(pending.len(), 2, "{pending:?}");
    assert!(
        pending
            .iter()
            .any(|entry| entry.path == "note.md" && entry.status == ChangeStatus::Modified),
        "{pending:?}"
    );
    assert!(
        pending
            .iter()
            .any(|entry| entry.path == "new.md" && entry.status == ChangeStatus::Added),
        "{pending:?}"
    );
    Ok(())
}

#[test]
fn git_import_command_apply_blocker_prevents_partial_pending_writes() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let vault_root = dir.path().join("vault");
    let repo_root = vault_root.join("default");
    {
        let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
        repo.set_projection_base_for_all_local_repos(&vault_root);
        init_git_repo(&repo_root);
        commit_deve_file(&dir, &repo, "note.md", "hello\n")?;
        git_cmd(&repo_root, &["add", "."]);
        git_cmd(&repo_root, &["commit", "--no-gpg-sign", "-m", "baseline"]);
    }

    write_workspace_file(&dir, "note.md", "hello import\n");
    write_workspace_file(&dir, "new.md", "new file\n");
    {
        let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
        repo.set_projection_base_for_all_local_repos(&vault_root);
        let doc_id = repo.get_docid("note.md")?;
        repo.run_on_local_repo("default", |db| {
            pending_fs::upsert(
                db,
                &PendingFsEntry {
                    path: "note.md".into(),
                    renamed_from: None,
                    doc_id,
                    change_type: ChangeStatus::Modified,
                    content_hash: pending_fs::content_hash("different pending"),
                    detected_at: 1,
                    has_conflict: false,
                },
            )
        })?;
    }

    git::import(&ledger_dir, Some("default"), true, 10)?;

    let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
    repo.set_projection_base_for_all_local_repos(&vault_root);
    let pending = repo.list_pending_fs_in_local_repo("default")?;
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert_eq!(pending[0].path, "note.md");
    assert!(!pending.iter().any(|entry| entry.path == "new.md"));
    Ok(())
}

#[test]
fn git_import_apply_resolved_commit_exports_roundtrip() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let vault_root = dir.path().join("vault");
    let repo_root = vault_root.join("default");
    prepare_exported_baseline(&dir, &ledger_dir, &vault_root, &repo_root)?;
    let imported_commit_id =
        resolve_imported_change_to_queued_commit(&dir, &ledger_dir, &vault_root)?;

    git::export(&ledger_dir, Some("default"), false, 10)?;
    assert_clean_resolved_import_export(&ledger_dir, &vault_root, &repo_root, &imported_commit_id)?;
    Ok(())
}

#[test]
fn git_import_export_push_resolved_publish_roundtrip() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let vault_root = dir.path().join("vault");
    let repo_root = vault_root.join("default");
    prepare_exported_baseline(&dir, &ledger_dir, &vault_root, &repo_root)?;
    let remote = dir.path().join("remote.git");
    init_bare_remote(&remote);
    git_cmd(
        &repo_root,
        &["remote", "add", "origin", remote.to_str().expect("remote")],
    );
    let branch = current_branch(&repo_root);
    let branch_ref = format!("refs/heads/{branch}");

    let imported_commit_id =
        resolve_imported_change_to_queued_commit(&dir, &ledger_dir, &vault_root)?;
    git::push(
        &ledger_dir,
        Some("default"),
        Some("origin"),
        Some(&branch),
        10,
    )?;
    let report = push_report(&ledger_dir, &vault_root, &repo_root, "origin", &branch)?;
    assert!(!report.pushed, "{report:?}");
    assert_push_blocker(&report, "git_worktree", "dirty Git worktree");
    assert_push_blocker(&report, "git_history_mapping", "unpublished mirror records");
    assert!(!git_success(
        &remote,
        &["rev-parse", "--verify", &branch_ref]
    ));

    git::export(&ledger_dir, Some("default"), false, 10)?;
    let git_commit_id = assert_clean_resolved_import_export(
        &ledger_dir,
        &vault_root,
        &repo_root,
        &imported_commit_id,
    )?;

    write_workspace_file(&dir, "dirty.md", "dirty\n");
    git::push(
        &ledger_dir,
        Some("default"),
        Some("origin"),
        Some(&branch),
        10,
    )?;
    let report = push_report(&ledger_dir, &vault_root, &repo_root, "origin", &branch)?;
    assert!(!report.pushed, "{report:?}");
    assert_push_blocker(&report, "git_worktree", "dirty Git worktree");
    assert!(!git_success(
        &remote,
        &["rev-parse", "--verify", &branch_ref]
    ));

    std::fs::remove_file(repo_root.join("dirty.md"))?;
    git::push(
        &ledger_dir,
        Some("default"),
        Some("origin"),
        Some(&branch),
        10,
    )?;
    assert_eq!(
        git_cmd(&remote, &["rev-parse", &branch_ref]).trim(),
        git_commit_id
    );
    Ok(())
}
