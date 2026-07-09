//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!
//! Remote source-control diff tests.

use super::remote_content::{local_counterpart_content, resolve_tracked_doc_id};
use super::remote_test_support::{
    commit_added_file, default_workspace_root, new_repo, pending_entry, seed_pending_entry,
    write_workspace_file,
};
use deve_core::ledger::schema::{DOCID_TO_PATH, NODEID_TO_META, PATH_TO_DOCID, PATH_TO_NODEID};
use deve_core::ledger::traits::RepoSelector;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeStatus, SourceControlApi};

#[test]
fn remote_diff_prefers_doc_id_for_local_counterpart() -> anyhow::Result<()> {
    let (dir, repo) = new_repo()?;
    let selector = RepoSelector::default();
    let doc_id = commit_added_file(&dir, &repo, "notes/a.md", "hello", "initial")?;

    std::fs::remove_file(default_workspace_root(&dir).join("notes/a.md"))?;
    write_workspace_file(&dir, "notes/b.md", "hello renamed");
    let mut added = pending_entry(
        "notes/b.md",
        Some(doc_id),
        ChangeStatus::Added,
        "hello renamed",
        2,
    );
    added.renamed_from = Some("notes/a.md".into());
    seed_pending_entry(
        &repo,
        pending_entry("notes/a.md", Some(doc_id), ChangeStatus::Deleted, "", 2),
    );
    seed_pending_entry(&repo, added);
    repo.stage_pending_in_repo(
        &selector,
        &ScPathTarget {
            path: "notes/b.md".into(),
            doc_id: Some(doc_id),
            domain: None,
        },
    )?;
    repo.apply_external_changes_in_repo(&selector)?;
    repo.commit_source_control_changes_in_repo(&selector, "rename")?;

    let content = local_counterpart_content(&repo, doc_id, repo.local_repo_name())?;
    assert_eq!(content.as_deref(), Some("hello renamed"));
    Ok(())
}

#[test]
fn remote_diff_prefers_node_projection_before_legacy_path_mapping() -> anyhow::Result<()> {
    let (dir, repo) = new_repo()?;
    commit_added_file(&dir, &repo, "notes/a.md", "hello", "initial")?;
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
    commit_added_file(&dir, &repo, "notes/a.md", "hello", "initial")?;
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
