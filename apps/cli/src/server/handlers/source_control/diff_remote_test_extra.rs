use super::remote::{local_counterpart_content, resolve_remote_content, resolve_tracked_doc_id};
use super::remote_test_support::{build_state, new_repo, seed_pending_entry, write_workspace_file};
use deve_core::ledger::schema::{NODEID_TO_META, PATH_TO_NODEID};
use deve_core::ledger::traits::{RepoSelector, Repository};
use deve_core::models::PeerId;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeStatus;
use deve_core::source_control::pending_fs::{self, PendingFsEntry};

#[test]
fn remote_diff_rejects_deleted_doc_even_with_doc_id_hint() -> anyhow::Result<()> {
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
    repo.apply_file_delete_structure_in_local_repo(
        repo.local_repo_name(),
        "notes/a.md",
        Some(doc_id),
        "test",
    )?;

    let resolved = repo.run_on_local_repo(repo.local_repo_name(), |db| {
        resolve_tracked_doc_id(
            db,
            &ScPathTarget {
                path: "notes/a.md".into(),
                doc_id: Some(doc_id),
            },
        )
    })?;
    assert!(resolved.is_none());
    assert!(local_counterpart_content(&repo, doc_id, Some(repo.local_repo_name()))?.is_none());
    Ok(())
}

#[test]
fn remote_diff_surfaces_shadow_lookup_errors_instead_of_not_found() -> anyhow::Result<()> {
    let (dir, repo) = new_repo()?;
    let peer = PeerId::new("peer-missing");
    let repo_id = uuid::Uuid::new_v4();
    std::fs::create_dir_all(
        dir.path()
            .join("remotes")
            .join(peer.to_filename())
            .join(format!("{}.redb", repo_id)),
    )?;
    let state = build_state(&dir, repo)?;
    let err = resolve_remote_content(
        &state,
        Some(&peer),
        repo_id,
        &ScPathTarget::from_path("notes/a.md"),
    )
    .expect_err("missing shadow repo should stay an error");
    let detail = err.to_string();
    assert!(
        detail.contains("shadow")
            || detail.contains("redb")
            || detail.contains("directory")
            || detail.contains("Is a directory"),
        "unexpected error detail: {detail}"
    );
    Ok(())
}

#[test]
fn remote_diff_fails_closed_when_local_doc_has_only_legacy_mapping() -> anyhow::Result<()> {
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

    let err = local_counterpart_content(&repo, doc_id, Some(repo.local_repo_name()))
        .expect_err("legacy-only local counterpart must fail closed");
    assert!(
        err.to_string()
            .contains("Tracked document projection missing for legacy-mapped doc")
    );
    Ok(())
}
