use deve_core::git_bridge::{self, GitMirrorCommitState};
use deve_core::ledger::RepoManager;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use redb::TableDefinition;
use tempfile::{TempDir, tempdir};

const BROKEN_GIT_MIRROR_TABLE: TableDefinition<u64, &str> =
    TableDefinition::new("git_mirror_commits");

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let ledger = dir.path().join("ledger");
    let projection_base = dir.path().join("notes");
    let mut repo = RepoManager::init(&ledger, 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(&projection_base)
        .expect("projection base");
    (dir, repo)
}

fn repo_root(repo: &RepoManager) -> std::path::PathBuf {
    repo.local_repo_workspace_root("default")
        .expect("workspace root")
}

fn write_workspace_file(repo: &RepoManager, path: &str, content: &str) {
    let abs = repo_root(repo).join(path);
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

fn seed_pending(repo: &RepoManager, path: &str, status: ChangeStatus, content: &str) {
    let doc_id = repo
        .get_tracked_docid_in_local_repo(repo.local_repo_name(), path)
        .expect("resolve tracked doc id for pending seed");
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
fn commit_queues_git_mirror_record_when_mirror_is_ready() {
    let (_dir, repo) = new_repo();
    let root = repo_root(&repo);
    std::fs::create_dir_all(root.join(".git")).expect("mkdir git");
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&root).expect("gitignore");
    write_workspace_file(&repo, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    repo.stage_pending("notes/a.md").expect("stage");
    repo.apply_external_changes()
        .expect("apply external change");

    let commit = repo
        .commit_source_control_changes("initial")
        .expect("commit");
    let repo_info = repo.get_repo_info().expect("repo info").expect("metadata");

    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            Ok(git_bridge::get_record(db, &commit.id)?)
        })
        .expect("read mirror record")
        .expect("mirror record");
    assert_eq!(record.deve_commit_id, commit.id);
    assert_eq!(record.repo_id, repo_info.uuid);
    assert_eq!(record.ledger_seq, commit.ledger_seq);
    assert_eq!(record.state, GitMirrorCommitState::Queued);
}

#[test]
fn commit_without_git_mirror_keeps_no_mirror_record() {
    let (_dir, repo) = new_repo();
    write_workspace_file(&repo, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    repo.stage_pending("notes/a.md").expect("stage");
    repo.apply_external_changes()
        .expect("apply external change");

    let commit = repo
        .commit_source_control_changes("initial")
        .expect("commit");

    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            Ok(git_bridge::get_record(db, &commit.id)?)
        })
        .expect("read mirror record");
    assert!(record.is_none());
}

#[test]
fn ngit_commit_always_queues_git_main_mirror_when_ready() {
    let (_dir, repo) = new_repo();
    let root = repo_root(&repo);
    std::fs::create_dir_all(root.join(".git")).expect("mkdir git");
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&root).expect("gitignore");
    write_workspace_file(&repo, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    repo.stage_pending("notes/a.md").expect("stage");
    repo.apply_external_changes()
        .expect("apply external change");

    let commit = repo
        .commit_source_control_changes("initial")
        .expect("commit");

    let record = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            Ok(git_bridge::get_record(db, &commit.id)?)
        })
        .expect("read mirror record");
    assert!(record.is_some());
    assert!(repo.list_staged().expect("staged after commit").is_empty());
    assert!(
        repo.list_commits(10)
            .expect("commits")
            .iter()
            .any(|item| item.id == commit.id)
    );
}

#[test]
fn git_mirror_queue_failure_does_not_rollback_deve_commit() {
    let (_dir, repo) = new_repo();
    let root = repo_root(&repo);
    std::fs::create_dir_all(root.join(".git")).expect("mkdir git");
    deve_core::utils::notegit::ensure_gitignore_ignores_notegit(&root).expect("gitignore");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let txn = db.begin_write()?;
        {
            let _ = txn.open_table(BROKEN_GIT_MIRROR_TABLE)?;
        }
        txn.commit()?;
        Ok(())
    })
    .expect("poison git mirror table type");
    write_workspace_file(&repo, "notes/a.md", "hello");
    seed_pending(&repo, "notes/a.md", ChangeStatus::Added, "hello");
    repo.stage_pending("notes/a.md").expect("stage");
    repo.apply_external_changes()
        .expect("apply external change");

    let commit = repo
        .commit_source_control_changes("initial despite mirror queue failure")
        .expect("commit must not roll back on mirror queue failure");

    assert!(repo.list_staged().expect("staged after commit").is_empty());
    let commits = repo.list_commits(10).expect("list commits");
    assert!(commits.iter().any(|item| item.id == commit.id));
    assert_eq!(
        std::fs::read_to_string(repo_root(&repo).join("notes/a.md")).expect("workspace file"),
        "hello"
    );
}
