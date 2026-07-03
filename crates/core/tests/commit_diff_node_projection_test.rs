use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::{DocId, LedgerEntry, NodeId, PeerId, StructureOp};
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::{TempDir, tempdir};

mod common;

fn new_repo() -> (TempDir, RepoManager) {
    let dir = tempdir().expect("create tempdir");
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
    repo.set_projection_base_for_all_local_repos_checked(dir.path().join("notes"))
        .expect("projection locator");
    (dir, repo)
}

fn write_workspace_file(repo: &RepoManager, path: &str, content: &str) {
    let abs = repo
        .local_repo_workspace_path("default", path)
        .expect("workspace path");
    if let Some(parent) = abs.parent() {
        std::fs::create_dir_all(parent).expect("create workspace parent");
    }
    std::fs::write(abs, content).expect("write workspace file");
}

#[test]
fn commit_diff_fails_closed_for_content_only_docs_without_structure_projection() {
    let (_dir, repo) = new_repo();
    let doc_id = deve_core::models::DocId::new();
    repo.append_local_op(&deve_core::models::LedgerEntry::new_content(
        doc_id,
        deve_core::models::Op::Insert {
            pos: 0,
            content: "orphan".into(),
        },
        0,
        deve_core::models::PeerId::new("test"),
        1,
        None,
        None,
    ))
    .expect("append orphan content op");
    let commit = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            let ledger_seq = deve_core::ledger::range::get_max_seq(db)?;
            deve_core::source_control::commits::create(db, "orphan-content", 1, ledger_seq)
        })
        .expect("create orphan commit");

    let err = repo
        .diff_commits(None, &commit.id)
        .expect_err("orphan content diff must fail closed");

    assert!(
        err.to_string()
            .contains("Commit diff lost projected path for doc")
    );
}

#[test]
fn commit_diff_fails_closed_on_missing_structure_targets() {
    let (_dir, repo) = new_repo();
    common::append_unvalidated_local_op(
        &repo,
        repo.local_repo_name(),
        &LedgerEntry::new_structure(
            StructureOp::MoveNode {
                node_id: NodeId::new(),
                doc_id: None,
                new_parent_id: None,
            },
            1,
            PeerId::new("test"),
            1,
        ),
    );
    let commit = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            let ledger_seq = deve_core::ledger::range::get_max_seq(db)?;
            deve_core::source_control::commits::create(db, "broken-structure", 0, ledger_seq)
        })
        .expect("create broken commit");

    let err = repo
        .diff_commits(None, &commit.id)
        .expect_err("broken structure diff must fail closed");
    assert!(err.to_string().contains("missing node"));
}

#[test]
fn commit_diff_fails_closed_when_doc_has_multiple_live_nodes() {
    let (_dir, repo) = new_repo();
    let doc_id = DocId::new();
    common::append_unvalidated_local_op(
        &repo,
        repo.local_repo_name(),
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::new(),
                doc_id,
                parent_id: None,
                name: "a.md".into(),
            },
            1,
            PeerId::new("test"),
            1,
        ),
    );
    common::append_unvalidated_local_op(
        &repo,
        repo.local_repo_name(),
        &LedgerEntry::new_structure(
            StructureOp::CreateFile {
                node_id: NodeId::new(),
                doc_id,
                parent_id: None,
                name: "b.md".into(),
            },
            1,
            PeerId::new("test"),
            2,
        ),
    );
    let commit = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            let ledger_seq = deve_core::ledger::range::get_max_seq(db)?;
            deve_core::source_control::commits::create(db, "duplicate-doc", 0, ledger_seq)
        })
        .expect("create duplicate-doc commit");

    let err = repo
        .diff_commits(None, &commit.id)
        .expect_err("duplicate live doc paths must fail closed");

    assert!(
        err.to_string().contains("multiple live paths"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn commit_diff_prefers_node_projection_path_over_stale_metadata() {
    let (_dir, repo) = new_repo();
    write_workspace_file(&repo, "notes/a.md", "v1");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("v1"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed initial add");
    repo.stage_pending("notes/a.md").expect("stage first");
    repo.apply_external_changes().expect("apply external first");
    let first = repo
        .commit_staged_with_git_bridge("first", deve_core::config::GitBridgeMode::Mirror)
        .expect("commit first");
    let doc_id = repo.get_docid("notes/a.md").expect("lookup").expect("doc");

    write_workspace_file(&repo, "notes/a.md", "v2");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("v2"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed modify");
    repo.stage_pending("notes/a.md").expect("stage second");
    repo.apply_external_changes()
        .expect("apply external second");
    let second = repo
        .commit_staged_with_git_bridge("second", deve_core::config::GitBridgeMode::Mirror)
        .expect("commit second");

    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write_txn = db.begin_write()?;
        {
            let mut p2d = write_txn.open_table(PATH_TO_DOCID)?;
            let mut d2p = write_txn.open_table(DOCID_TO_PATH)?;
            p2d.remove("notes/a.md")?;
            p2d.insert("stale/a.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "stale/a.md")?;
        }
        write_txn.commit()?;
        Ok(())
    })
    .expect("poison metadata only");

    let diffs = repo
        .diff_commits(Some(&first.id), &second.id)
        .expect("diff commits");
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].doc_id, Some(doc_id));
    assert_eq!(diffs[0].path, "notes/a.md");
    assert_eq!(diffs[0].status, ChangeStatus::Modified);
}

#[test]
fn commit_diff_rejects_reversed_commit_order() {
    let (_dir, repo) = new_repo();
    write_workspace_file(&repo, "notes/a.md", "v1");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("v1"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed initial add");
    repo.stage_pending("notes/a.md").expect("stage first");
    repo.apply_external_changes().expect("apply external first");
    let first = repo
        .commit_staged_with_git_bridge("first", deve_core::config::GitBridgeMode::Mirror)
        .expect("commit first");
    let doc_id = repo.get_docid("notes/a.md").expect("lookup").expect("doc");

    write_workspace_file(&repo, "notes/a.md", "v2");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("v2"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed modify");
    repo.stage_pending("notes/a.md").expect("stage second");
    repo.apply_external_changes()
        .expect("apply external second");
    let second = repo
        .commit_staged_with_git_bridge("second", deve_core::config::GitBridgeMode::Mirror)
        .expect("commit second");

    let err = repo
        .diff_commits(Some(&second.id), &first.id)
        .expect_err("reversed commit order must fail closed");

    assert!(
        err.to_string().contains("invalid order"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn commit_diff_reports_rename_from_structure_facts() {
    let (_dir, repo) = new_repo();
    write_workspace_file(&repo, "notes/a.md", "v1");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("v1"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })
    .expect("seed initial add");
    repo.stage_pending("notes/a.md").expect("stage first");
    repo.apply_external_changes().expect("apply external first");
    let first = repo
        .commit_staged_with_git_bridge("first", deve_core::config::GitBridgeMode::Mirror)
        .expect("commit first");
    let doc_id = repo.get_docid("notes/a.md").expect("lookup").expect("doc");

    write_workspace_file(&repo, "notes/b.md", "v1");
    std::fs::remove_file(
        repo.local_repo_workspace_path("default", "notes/a.md")
            .expect("workspace path"),
    )
    .expect("remove old path");
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/a.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Deleted,
                content_hash: String::new(),
                detected_at: 2,
                has_conflict: false,
            },
        )?;
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/b.md".into(),
                renamed_from: Some("notes/a.md".into()),
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Added,
                content_hash: pending_fs::content_hash("v1"),
                detected_at: 2,
                has_conflict: false,
            },
        )
    })
    .expect("seed rename");
    repo.stage_pending("notes/a.md").expect("stage delete");
    repo.stage_pending("notes/b.md").expect("stage add");
    repo.apply_external_changes()
        .expect("apply external rename");
    let second = repo
        .commit_staged_with_git_bridge("rename", deve_core::config::GitBridgeMode::Mirror)
        .expect("commit rename");

    let diffs = repo
        .diff_commits(Some(&first.id), &second.id)
        .expect("diff commits");
    assert_eq!(diffs.len(), 1);
    assert_eq!(diffs[0].doc_id, Some(doc_id));
    assert_eq!(diffs[0].path, "notes/b.md");
    assert_eq!(diffs[0].previous_path.as_deref(), Some("notes/a.md"));
    assert_eq!(diffs[0].status, ChangeStatus::Renamed);
    assert_eq!(diffs[0].old_content, "v1");
    assert_eq!(diffs[0].new_content, "v1");
}
