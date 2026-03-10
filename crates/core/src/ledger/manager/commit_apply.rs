use crate::ledger::RepoManager;
use crate::source_control::changes;
use crate::sync::reconcile;
use anyhow::Result;

impl RepoManager {
    pub(super) fn commit_file_ops_in_local_repo(
        &self,
        repo_name: &str,
        _vault_root: &std::path::Path,
        normalized_path: &str,
        doc_id_hint: Option<crate::models::DocId>,
    ) -> Result<()> {
        let doc_id = self.apply_file_structure_in_local_repo(
            repo_name,
            normalized_path,
            doc_id_hint,
            "local_commit",
        )?;
        let disk_path = self.local_repo_workspace_path(repo_name, normalized_path)?;
        let disk_content = std::fs::read_to_string(&disk_path).unwrap_or_default();
        let existing_ops = self.get_local_ops_in_local_repo(repo_name, doc_id)?;
        let entries: Vec<_> = existing_ops.into_iter().map(|(_, entry)| entry).collect();
        let patch = reconcile::compute_reconcile_patch(&entries, &disk_content)?;
        reconcile::append_patch_in_local_repo(self, repo_name, doc_id, "local_commit", &patch)?;
        self.run_on_local_repo(repo_name, |db| {
            changes::save_snapshot(db, doc_id, normalized_path, &disk_content)
        })
    }

    pub(super) fn commit_delete_snapshot_in_local_repo(
        &self,
        repo_name: &str,
        normalized_path: &str,
        doc_id_hint: Option<crate::models::DocId>,
    ) -> Result<()> {
        let Some(doc_id) = self.apply_file_delete_structure_in_local_repo(
            repo_name,
            normalized_path,
            doc_id_hint,
            "local_commit",
        )?
        else {
            return Ok(());
        };
        self.run_on_local_repo(repo_name, |db| changes::remove_snapshot(db, doc_id))
    }
}
