use deve_core::ledger::RepoManager;
use deve_core::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
use deve_core::models::DocId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};
use tempfile::tempdir;

#[test]
fn repo_discard_target_fails_closed_when_doc_id_does_not_match() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos(dir.path().join("notes"));
    let (doc_id, _ops) =
        repo.apply_file_structure_in_local_repo("default", "notes/live.md", None, "test")?;
    repo.run_on_local_repo("default", |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/live.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("dirty"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;

    let err = repo
        .discard_pending_target_in_local_repo(
            "default",
            &ScPathTarget {
                path: "notes/live.md".into(),
                doc_id: Some(DocId::new()),
            },
        )
        .expect_err("mismatched doc target must fail closed");

    assert!(err.to_string().contains("Path is not in pending_fs_ops"));
    Ok(())
}

#[test]
fn repo_discard_target_does_not_fallback_to_docless_pending_when_doc_id_misses()
-> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos(dir.path().join("notes"));
    let (doc_id, _ops) =
        repo.apply_file_structure_in_local_repo("default", "notes/live.md", None, "test")?;
    let wrong_doc_id = DocId::new();
    assert_ne!(doc_id, wrong_doc_id);
    repo.run_on_local_repo("default", |db| {
        pending_fs::upsert(
            db,
            &PendingFsEntry {
                path: "notes/live.md".into(),
                renamed_from: None,
                doc_id: None,
                change_type: ChangeStatus::Modified,
                content_hash: pending_fs::content_hash("dirty"),
                detected_at: 1,
                has_conflict: false,
            },
        )
    })?;

    let err = repo
        .discard_pending_target_in_local_repo(
            "default",
            &ScPathTarget {
                path: "notes/live.md".into(),
                doc_id: Some(wrong_doc_id),
            },
        )
        .expect_err("doc_id miss must not fall back to path-only pending");
    let pending = repo.run_on_local_repo("default", pending_fs::list_all)?;

    assert!(err.to_string().contains("Path is not in pending_fs_ops"));
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].path, "notes/live.md");
    assert_eq!(pending[0].doc_id, None);
    Ok(())
}

#[test]
fn repo_diff_target_fails_closed_when_path_is_not_tracked_or_changed() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos(dir.path().join("notes"));

    let err = repo
        .diff_doc_target_in_local_repo("default", &ScPathTarget::from_path("notes/missing.md"))
        .expect_err("untracked path-only diff target must fail closed");

    assert!(
        err.to_string()
            .contains("Source control target not resolved for path notes/missing.md")
    );
    Ok(())
}

#[test]
fn repo_diff_path_returns_not_found_when_only_legacy_path_mapping_exists() -> anyhow::Result<()> {
    let dir = tempdir()?;
    let mut repo = RepoManager::init(dir.path().join("ledger"), 10, None, None)?;
    repo.set_projection_base_for_all_local_repos(dir.path().join("notes"));
    let doc_id = DocId::new();
    repo.run_on_local_repo("default", |db| {
        let write = db.begin_write()?;
        {
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            p2d.insert("notes/legacy.md", doc_id.as_u128())?;
            d2p.insert(doc_id.as_u128(), "notes/legacy.md")?;
        }
        write.commit()?;
        Ok::<_, anyhow::Error>(())
    })?;

    let err = repo
        .diff_doc_path("notes/legacy.md")
        .expect_err("legacy-only path must fail with doc not found");

    assert!(
        err.to_string().contains("Doc not found") || err.to_string().contains("not resolved"),
        "expected not-found error, got: {}",
        err
    );
    Ok(())
}
