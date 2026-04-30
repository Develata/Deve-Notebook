use super::git;
use anyhow::Result;
use deve_core::ledger::RepoManager;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git_cmd(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_git_repo(path: &Path) {
    std::fs::create_dir_all(path).expect("repo dir");
    git_cmd(path, &["init"]);
    git_cmd(path, &["config", "user.email", "deve@example.invalid"]);
    git_cmd(path, &["config", "user.name", "Deve Test"]);
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(path).expect("gitignore");
}

fn write_workspace_file(dir: &TempDir, path: &str, content: &str) {
    let abs = dir.path().join("vault/default").join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn commit_deve_file(dir: &TempDir, repo: &RepoManager, path: &str, content: &str) -> Result<()> {
    write_workspace_file(dir, path, content);
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: path.into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash(content),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;
    repo.stage_pending(path)?;
    repo.commit_staged("initial")?;
    Ok(())
}

#[test]
fn git_import_command_dry_run_is_read_only_and_apply_writes_pending() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let vault_root = dir.path().join("vault");
    let repo_root = vault_root.join("default");
    {
        let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
        repo.set_vault_root(&vault_root);
        init_git_repo(&repo_root);
        commit_deve_file(&dir, &repo, "note.md", "hello\n")?;
        git_cmd(&repo_root, &["add", "."]);
        git_cmd(&repo_root, &["commit", "--no-gpg-sign", "-m", "baseline"]);
    }

    write_workspace_file(&dir, "note.md", "hello import\n");
    write_workspace_file(&dir, "new.md", "new file\n");

    git::import(&ledger_dir, &vault_root, Some("default"), false, 10)?;
    {
        let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
        repo.set_vault_root(&vault_root);
        let pending = repo.list_pending_fs_in_local_repo("default")?;
        assert!(pending.is_empty(), "{pending:?}");
    }

    git::import(&ledger_dir, &vault_root, Some("default"), true, 10)?;
    let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
    repo.set_vault_root(&vault_root);
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
        repo.set_vault_root(&vault_root);
        init_git_repo(&repo_root);
        commit_deve_file(&dir, &repo, "note.md", "hello\n")?;
        git_cmd(&repo_root, &["add", "."]);
        git_cmd(&repo_root, &["commit", "--no-gpg-sign", "-m", "baseline"]);
    }

    write_workspace_file(&dir, "note.md", "hello import\n");
    write_workspace_file(&dir, "new.md", "new file\n");
    {
        let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
        repo.set_vault_root(&vault_root);
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

    git::import(&ledger_dir, &vault_root, Some("default"), true, 10)?;

    let mut repo = RepoManager::init(&ledger_dir, 10, None, None)?;
    repo.set_vault_root(&vault_root);
    let pending = repo.list_pending_fs_in_local_repo("default")?;
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert_eq!(pending[0].path, "note.md");
    assert!(!pending.iter().any(|entry| entry.path == "new.md"));
    Ok(())
}
