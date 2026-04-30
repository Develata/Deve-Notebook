use super::git;
use anyhow::Result;
use deve_core::git_bridge::{GitMirrorCommitState, get_record};
use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn git_cmd(path: &Path, args: &[&str]) -> String {
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
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn git_success(path: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .expect("run git")
        .status
        .success()
}

fn init_git_repo(path: &Path) {
    std::fs::create_dir_all(path).expect("repo dir");
    git_cmd(path, &["init"]);
    git_cmd(path, &["config", "user.email", "deve@example.invalid"]);
    git_cmd(path, &["config", "user.name", "Deve Test"]);
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(path).expect("gitignore");
}

fn init_bare_remote(path: &Path) {
    std::fs::create_dir_all(path).expect("remote dir");
    git_cmd(path, &["init", "--bare"]);
}

fn current_branch(repo_root: &Path) -> String {
    git_cmd(repo_root, &["branch", "--show-current"])
        .trim()
        .to_string()
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

fn open_repo(ledger_dir: &Path, vault_root: &Path) -> Result<RepoManager> {
    let mut repo = RepoManager::init(ledger_dir, 10, None, None)?;
    repo.set_vault_root(vault_root);
    Ok(repo)
}

fn prepare_exported_baseline(
    dir: &TempDir,
    ledger_dir: &Path,
    vault_root: &Path,
    repo_root: &Path,
) -> Result<()> {
    {
        let repo = open_repo(ledger_dir, vault_root)?;
        init_git_repo(repo_root);
        commit_deve_file(dir, &repo, "note.md", "hello\n")?;
    }
    git::export(ledger_dir, vault_root, Some("default"), false, 10)?;
    assert_eq!(git_cmd(repo_root, &["show", "HEAD:note.md"]), "hello\n");
    assert!(git_cmd(repo_root, &["status", "--porcelain"]).is_empty());
    Ok(())
}

fn resolve_imported_change_to_queued_commit(
    dir: &TempDir,
    ledger_dir: &Path,
    vault_root: &Path,
) -> Result<String> {
    let doc_id = {
        let repo = open_repo(ledger_dir, vault_root)?;
        let doc_id = repo
            .get_tracked_docid_in_local_repo("default", "note.md")?
            .expect("doc id");
        repo.append_generated_op_in_local_repo("default", doc_id, PeerId::new("local"), |seq| {
            LedgerEntry::new_content(
                doc_id,
                Op::Insert {
                    pos: 6,
                    content: "ledger\n".into(),
                },
                2,
                PeerId::new("local"),
                seq,
                None,
                None,
            )
        })?;
        doc_id
    };
    write_workspace_file(dir, "note.md", "git import\n");
    git::import(ledger_dir, vault_root, Some("default"), true, 10)?;

    let repo = open_repo(ledger_dir, vault_root)?;
    let pending = repo.list_pending_fs_in_local_repo("default")?;
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert_eq!(pending[0].path, "note.md");
    assert!(pending[0].has_conflict, "{pending:?}");
    repo.stage_resolved_pending_target_in_local_repo(
        "default",
        &ScPathTarget {
            path: "note.md".into(),
            doc_id: Some(doc_id),
        },
    )?;
    let staged = repo.list_staged_in_local_repo("default")?;
    assert_eq!(staged.len(), 1, "{staged:?}");
    assert_eq!(staged[0].path, "note.md");
    assert!(!staged[0].has_conflict, "{staged:?}");
    let commit = repo.commit_staged_in_local_repo("default", "accept imported git content")?;
    assert!(repo.list_pending_fs_in_local_repo("default")?.is_empty());
    assert!(repo.list_staged_in_local_repo("default")?.is_empty());
    repo.run_on_local_repo("default", |db| {
        let record = get_record(db, &commit.id)?.expect("queued imported commit");
        assert_eq!(record.state, GitMirrorCommitState::Queued);
        Ok::<_, anyhow::Error>(())
    })?;
    Ok(commit.id)
}

fn exported_git_commit_id(
    ledger_dir: &Path,
    vault_root: &Path,
    deve_commit_id: &str,
) -> Result<String> {
    let repo = open_repo(ledger_dir, vault_root)?;
    repo.run_on_local_repo("default", |db| {
        let record = get_record(db, deve_commit_id)?.expect("committed imported commit");
        assert_eq!(record.state, GitMirrorCommitState::Committed);
        record
            .git_commit_id
            .ok_or_else(|| anyhow::anyhow!("missing Git commit id for {deve_commit_id}"))
    })
}

fn assert_clean_resolved_import_export(
    ledger_dir: &Path,
    vault_root: &Path,
    repo_root: &Path,
    deve_commit_id: &str,
) -> Result<String> {
    let git_commit_id = exported_git_commit_id(ledger_dir, vault_root, deve_commit_id)?;
    let repo = open_repo(ledger_dir, vault_root)?;
    assert!(repo.list_pending_fs_in_local_repo("default")?.is_empty());
    assert!(repo.list_staged_in_local_repo("default")?.is_empty());
    assert!(git_cmd(repo_root, &["status", "--porcelain"]).is_empty());
    assert_eq!(
        git_cmd(repo_root, &["show", "HEAD:note.md"]),
        "git import\n"
    );
    let head_body = git_cmd(repo_root, &["log", "-1", "--format=%B"]);
    assert!(head_body.contains(deve_commit_id), "{head_body}");
    Ok(git_commit_id)
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

#[test]
fn git_import_apply_resolved_commit_exports_roundtrip() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let ledger_dir = dir.path().join("ledger");
    let vault_root = dir.path().join("vault");
    let repo_root = vault_root.join("default");
    prepare_exported_baseline(&dir, &ledger_dir, &vault_root, &repo_root)?;
    let imported_commit_id =
        resolve_imported_change_to_queued_commit(&dir, &ledger_dir, &vault_root)?;

    git::export(&ledger_dir, &vault_root, Some("default"), false, 10)?;
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
        &vault_root,
        Some("default"),
        Some("origin"),
        Some(&branch),
        10,
    )?;
    assert!(!git_success(
        &remote,
        &["rev-parse", "--verify", &branch_ref]
    ));

    git::export(&ledger_dir, &vault_root, Some("default"), false, 10)?;
    let git_commit_id = assert_clean_resolved_import_export(
        &ledger_dir,
        &vault_root,
        &repo_root,
        &imported_commit_id,
    )?;

    write_workspace_file(&dir, "dirty.md", "dirty\n");
    git::push(
        &ledger_dir,
        &vault_root,
        Some("default"),
        Some("origin"),
        Some(&branch),
        10,
    )?;
    assert!(!git_success(
        &remote,
        &["rev-parse", "--verify", &branch_ref]
    ));

    std::fs::remove_file(repo_root.join("dirty.md"))?;
    git::push(
        &ledger_dir,
        &vault_root,
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
