//! plan_ref:
//!   - 03_storage/index#git-ecosystem-coexistence
//!   - 05_diff_logic#git-mirror-lifecycle
//!   - 14_commands#cli-commands
//!
//! Test support for the explicit Git mirror import/export/push command surface.

use super::git;
use anyhow::Result;
use deve_core::git_bridge::{
    GitMirrorCommitState, GitMirrorPushOptions, GitMirrorPushReport, get_record, push_mirror,
};
use deve_core::ledger::RepoManager;
use deve_core::models::{LedgerEntry, Op, PeerId};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use std::path::Path;
use std::process::Command;

pub(super) fn git_cmd(path: &Path, args: &[&str]) -> String {
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

pub(super) fn git_success(path: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .expect("run git")
        .status
        .success()
}

pub(super) fn init_git_repo(path: &Path) {
    std::fs::create_dir_all(path).expect("repo dir");
    git_cmd(path, &["init"]);
    git_cmd(path, &["config", "user.email", "deve@example.invalid"]);
    git_cmd(path, &["config", "user.name", "Deve Test"]);
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(path).expect("gitignore");
}

pub(super) fn init_bare_remote(path: &Path) {
    std::fs::create_dir_all(path).expect("remote dir");
    git_cmd(path, &["init", "--bare"]);
}

pub(super) fn current_branch(repo_root: &Path) -> String {
    git_cmd(repo_root, &["branch", "--show-current"])
        .trim()
        .to_string()
}

pub(super) fn write_workspace_file(repo_root: &Path, path: &str, content: &str) {
    let abs = repo_root.join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

pub(super) fn commit_deve_file(
    repo_root: &Path,
    repo: &RepoManager,
    path: &str,
    content: &str,
) -> Result<()> {
    write_workspace_file(repo_root, path, content);
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
    repo.commit_source_control_changes("initial")?;
    Ok(())
}

pub(super) fn open_repo(ledger_dir: &Path, projection_base: &Path) -> Result<RepoManager> {
    let mut repo = RepoManager::init(ledger_dir, 10, None, None)?;
    repo.set_projection_base_for_all_local_repos_checked(projection_base)?;
    Ok(repo)
}

pub(super) fn prepare_exported_baseline(
    ledger_dir: &Path,
    projection_base: &Path,
    repo_root: &Path,
) -> Result<()> {
    {
        let repo = open_repo(ledger_dir, projection_base)?;
        init_git_repo(repo_root);
        commit_deve_file(repo_root, &repo, "note.md", "hello\n")?;
    }
    git::export(ledger_dir, Some("default"), false, 10)?;
    assert_eq!(git_cmd(repo_root, &["show", "HEAD:note.md"]), "hello\n");
    assert!(git_cmd(repo_root, &["status", "--porcelain"]).is_empty());
    Ok(())
}

pub(super) fn resolve_imported_change_to_queued_commit(
    ledger_dir: &Path,
    projection_base: &Path,
) -> Result<String> {
    let doc_id = {
        let repo = open_repo(ledger_dir, projection_base)?;
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
    let repo = open_repo(ledger_dir, projection_base)?;
    let repo_root = repo.local_repo_workspace_root("default")?;
    write_workspace_file(&repo_root, "note.md", "git import\n");
    git::import(ledger_dir, Some("default"), true, 10)?;

    let repo = open_repo(ledger_dir, projection_base)?;
    let pending = repo.list_pending_fs_in_local_repo("default")?;
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert_eq!(pending[0].path, "note.md");
    assert!(pending[0].has_conflict, "{pending:?}");
    repo.stage_resolved_pending_target_in_local_repo(
        "default",
        &ScPathTarget {
            path: "note.md".into(),
            doc_id: Some(doc_id),
            domain: None,
        },
    )?;
    let staged = repo.list_staged_in_local_repo("default")?;
    assert_eq!(staged.len(), 1, "{staged:?}");
    assert_eq!(staged[0].path, "note.md");
    assert!(!staged[0].has_conflict, "{staged:?}");
    let commit =
        repo.commit_source_control_changes_in_local_repo("default", "accept imported git content")?;
    assert!(repo.list_pending_fs_in_local_repo("default")?.is_empty());
    assert!(repo.list_staged_in_local_repo("default")?.is_empty());
    repo.run_on_local_repo("default", |db| {
        let record = get_record(db, &commit.id)?.expect("queued imported commit");
        assert_eq!(record.state, GitMirrorCommitState::Queued);
        Ok::<_, anyhow::Error>(())
    })?;
    Ok(commit.id)
}

pub(super) fn exported_git_commit_id(
    ledger_dir: &Path,
    projection_base: &Path,
    deve_commit_id: &str,
) -> Result<String> {
    let repo = open_repo(ledger_dir, projection_base)?;
    repo.run_on_local_repo("default", |db| {
        let record = get_record(db, deve_commit_id)?.expect("committed imported commit");
        assert_eq!(record.state, GitMirrorCommitState::Committed);
        record
            .git_commit_id
            .ok_or_else(|| anyhow::anyhow!("missing Git commit id for {deve_commit_id}"))
    })
}

pub(super) fn assert_clean_resolved_import_export(
    ledger_dir: &Path,
    projection_base: &Path,
    repo_root: &Path,
    deve_commit_id: &str,
) -> Result<String> {
    let git_commit_id = exported_git_commit_id(ledger_dir, projection_base, deve_commit_id)?;
    let repo = open_repo(ledger_dir, projection_base)?;
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

pub(super) fn push_report(
    ledger_dir: &Path,
    projection_base: &Path,
    repo_root: &Path,
    remote: &str,
    branch: &str,
) -> Result<GitMirrorPushReport> {
    let repo = open_repo(ledger_dir, projection_base)?;
    repo.run_on_local_repo("default", |db| {
        Ok(push_mirror(
            db,
            repo_root,
            GitMirrorPushOptions {
                remote: Some(remote.into()),
                branch: Some(branch.into()),
            },
        )?)
    })
}

pub(super) fn assert_push_blocker(
    report: &GitMirrorPushReport,
    location: &str,
    reason_fragment: &str,
) {
    assert!(
        report.blockers.iter().any(|blocker| {
            blocker.location == location && blocker.reason.contains(reason_fragment)
        }),
        "{report:?}"
    );
}
