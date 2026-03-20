use super::remote::{local_counterpart_content, resolve_tracked_doc_id};
use super::remote_test_support::{new_repo, seed_pending_entry, write_workspace_file};
use deve_core::ledger::schema::{DOCID_TO_PATH, NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID};
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};

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

    let content = local_counterpart_content(&repo, doc_id, repo.local_repo_name())?;
    assert_eq!(content.as_deref(), Some("hello renamed"));
    Ok(())
}

#[test]
fn remote_diff_prefers_node_projection_before_legacy_path_mapping() -> anyhow::Result<()> {
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
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        {
            let mut d2p = write.open_table(DOCID_TO_PATH)?;
            let mut p2d = write.open_table(PATH_TO_DOCID)?;
            d2p.retain(|_, _| false)?;
            p2d.retain(|_, _| false)?;
        }
        write.commit()?;
        Ok(())
    })?;

    let doc_id = repo.run_on_local_repo(repo.local_repo_name(), |db| {
        resolve_tracked_doc_id(db, &ScPathTarget::from_path("notes/a.md"))
    })?;
    assert!(doc_id.is_some());
    Ok(())
}

#[test]
fn remote_diff_fails_closed_on_legacy_only_path_mapping() -> anyhow::Result<()> {
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
    repo.run_on_local_repo(repo.local_repo_name(), |db| {
        let write = db.begin_write()?;
        {
            let mut p2n = write.open_table(PATH_TO_NODEID)?;
            let mut n2m = write.open_table(NODEID_TO_META)?;
            p2n.retain(|_, _| false)?;
            n2m.retain(|_, _| false)?;
        }
        write.commit()?;
        Ok(())
    })?;

    let err = repo
        .run_on_local_repo(repo.local_repo_name(), |db| {
            resolve_tracked_doc_id(db, &ScPathTarget::from_path("notes/a.md"))
        })
        .expect_err("legacy-only path mapping must fail closed");
    assert!(
        err.to_string()
            .contains("Tracked document projection missing for legacy-mapped path")
    );
    Ok(())
}
