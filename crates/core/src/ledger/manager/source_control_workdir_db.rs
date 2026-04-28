//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 04_storage#projection-contract
//!   - 06_repository#tree-projection-contract
//!
use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::pending_fs;
use anyhow::Result;

use super::projection_cleanup::drop_unanchored_projection_path;

impl RepoManager {
    pub(crate) fn discard_untracked_pending_add_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<()> {
        self.get_tracked_docid_in_local_repo(repo_name, path)?;
        self.run_on_local_repo(repo_name, |db| {
            pending_fs::remove(db, path)?;
            drop_unanchored_projection_path(db, path)?;
            Ok(())
        })
    }

    pub(crate) fn clear_pending_for_doc_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        path: &str,
    ) -> Result<()> {
        self.run_on_local_repo(repo_name, |db| clear_pending_for_doc(db, doc_id, path))
    }
}

fn clear_pending_for_doc(db: &redb::Database, doc_id: DocId, path: &str) -> Result<()> {
    let mut paths = vec![path.to_string()];
    for entry in pending_fs::list_for_doc(db, doc_id)? {
        if entry.doc_id == Some(doc_id) && !paths.iter().any(|item| item == &entry.path) {
            paths.push(entry.path);
        }
    }
    for path in paths {
        pending_fs::remove(db, &path)?;
    }
    Ok(())
}
