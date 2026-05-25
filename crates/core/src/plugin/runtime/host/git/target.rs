//! plan_ref:
//!   - 19_plugins#plugin-runtime-boundary
//!
use crate::ledger::RepoManager;
use crate::protocol::ScPathTarget;
use crate::utils::path::to_forward_slash;
use rhai::EvalAltResult;

pub(super) fn resolve_local_sc_target(
    repo_manager: &RepoManager,
    path: &str,
) -> Result<ScPathTarget, Box<EvalAltResult>> {
    let path = to_forward_slash(path);
    repo_manager
        .tracked_target_for_path_in_local_repo(repo_manager.local_repo_name(), &path)
        .map_err(|e: anyhow::Error| e.to_string().into())
}

#[cfg(test)]
mod tests {
    use super::resolve_local_sc_target;
    use crate::ledger::schema::{DOCID_TO_PATH, PATH_TO_DOCID};
    use crate::ledger::{RepoManager, node_meta};
    use crate::models::DocId;
    use crate::source_control::{ChangeStatus, pending_fs};
    use tempfile::tempdir;

    fn new_repo() -> anyhow::Result<(tempfile::TempDir, RepoManager)> {
        let dir = tempdir()?;
        let ledger = dir.path().join("ledger");
        let projection_base = dir.path().join("notes");
        let mut repo = RepoManager::init(&ledger, 10, None, None)?;
        repo.set_projection_base_for_all_local_repos_checked(&projection_base)?;
        Ok((dir, repo))
    }

    #[test]
    fn resolve_local_sc_target_returns_none_for_legacy_only_path_mapping() -> anyhow::Result<()> {
        let (_dir, repo) = new_repo()?;
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

        let target = resolve_local_sc_target(&repo, "notes/legacy.md")
            .expect("legacy-only path returns Ok with no doc_id");

        assert_eq!(target.path, "notes/legacy.md");
        assert_eq!(
            target.doc_id, None,
            "legacy-only mapping must not resolve to doc_id"
        );
        Ok(())
    }

    #[test]
    fn resolve_local_sc_target_fills_doc_id_from_tracked_projection() -> anyhow::Result<()> {
        let (_dir, repo) = new_repo()?;
        let doc_id = DocId::new();
        repo.run_on_local_repo("default", |db| {
            node_meta::ensure_file_node(db, "notes/a.md", doc_id)?;
            pending_fs::upsert(
                db,
                &pending_fs::PendingFsEntry {
                    path: "notes/a.md".into(),
                    renamed_from: None,
                    doc_id: None,
                    change_type: ChangeStatus::Modified,
                    content_hash: "hash".into(),
                    detected_at: 0,
                    has_conflict: false,
                },
            )?;
            Ok::<_, anyhow::Error>(())
        })?;

        let target = resolve_local_sc_target(&repo, "notes/a.md").expect("resolved");
        assert_eq!(target.path, "notes/a.md");
        assert_eq!(target.doc_id, Some(doc_id));
        Ok(())
    }

    #[test]
    fn resolve_local_sc_target_fails_closed_when_old_path_is_reused() -> anyhow::Result<()> {
        let (_dir, repo) = new_repo()?;
        let doc_id = crate::models::DocId::new();
        repo.run_on_local_repo(repo.local_repo_name(), |db| {
            pending_fs::upsert(
                db,
                &pending_fs::PendingFsEntry {
                    path: "docs/a.md".into(),
                    renamed_from: Some("notes/a.md".into()),
                    doc_id: Some(doc_id),
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("hello"),
                    detected_at: 1,
                    has_conflict: false,
                },
            )?;
            pending_fs::upsert(
                db,
                &pending_fs::PendingFsEntry {
                    path: "notes/a.md".into(),
                    renamed_from: None,
                    doc_id: None,
                    change_type: ChangeStatus::Added,
                    content_hash: pending_fs::content_hash("new"),
                    detected_at: 2,
                    has_conflict: false,
                },
            )
        })?;

        let err = resolve_local_sc_target(&repo, "notes/a.md")
            .expect_err("reused old path must fail closed");
        assert!(
            err.to_string()
                .contains("Ambiguous source control path target: notes/a.md")
        );
        Ok(())
    }
}
