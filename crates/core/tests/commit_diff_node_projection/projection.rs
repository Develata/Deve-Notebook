use super::*;

#[test]
fn commit_diff_prefers_node_projection_path_over_stale_metadata() {
    let (dir, repo) = new_repo();
    write_workspace_file(&dir, "notes/a.md", "v1");
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
    let first = repo.commit_staged("first").expect("commit first");
    let doc_id = repo.get_docid("notes/a.md").expect("lookup").expect("doc");

    write_workspace_file(&dir, "notes/a.md", "v2");
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
    let second = repo.commit_staged("second").expect("commit second");

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
fn commit_diff_reports_rename_from_structure_facts() {
    let (dir, repo) = new_repo();
    write_workspace_file(&dir, "notes/a.md", "v1");
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
    let first = repo.commit_staged("first").expect("commit first");
    let doc_id = repo.get_docid("notes/a.md").expect("lookup").expect("doc");

    write_workspace_file(&dir, "notes/b.md", "v1");
    std::fs::remove_file(dir.path().join("vault/default/notes/a.md")).expect("remove old path");
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
    let second = repo.commit_staged("rename").expect("commit rename");

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
