use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::pending_fs;
use anyhow::Result;

use super::projection_cleanup::drop_unanchored_projection_path;
use super::source_control_workdir_helpers::clear_pending_for_doc;

impl RepoManager {
    pub(crate) fn discard_untracked_pending_add_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<()> {
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
