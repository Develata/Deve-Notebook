use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::source_control::changes;
use crate::sync::reconcile;
use anyhow::Result;

impl RepoManager {
    pub(super) fn commit_file_ops_in_local_repo(
        &self,
        repo_name: &str,
        _vault_root: &std::path::Path,
        normalized_path: &str,
        doc_id_hint: Option<DocId>,
    ) -> Result<()> {
        let doc_id =
            self.resolve_commit_doc_id_in_local_repo(repo_name, normalized_path, doc_id_hint)?;
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
        doc_id_hint: Option<DocId>,
    ) -> Result<()> {
        use crate::source_control::snapshot_paths;
        let doc_id = match doc_id_hint {
            Some(doc_id) => doc_id,
            None => match self.run_on_local_repo(repo_name, |db| {
                snapshot_paths::find_snapshot_doc_id(db, normalized_path)
            })? {
                Some(doc_id) => doc_id,
                None => return Ok(()),
            },
        };
        if let Some(current_path) = self.get_path_by_docid_in_local_repo(repo_name, doc_id)?
            && current_path != normalized_path
        {
            return Ok(());
        }
        let existing_ops = self.get_local_ops_in_local_repo(repo_name, doc_id)?;
        let entries: Vec<_> = existing_ops.into_iter().map(|(_, entry)| entry).collect();
        let patch = reconcile::compute_reconcile_patch(&entries, "")?;
        reconcile::append_patch_in_local_repo(self, repo_name, doc_id, "local_commit", &patch)?;
        self.run_on_local_repo(repo_name, |db| {
            changes::remove_snapshot(db, doc_id)?;
            crate::ledger::metadata::delete_doc(db, normalized_path)
        })
    }

    fn resolve_commit_doc_id_in_local_repo(
        &self,
        repo_name: &str,
        normalized_path: &str,
        doc_id_hint: Option<DocId>,
    ) -> Result<DocId> {
        if let Some(doc_id) = doc_id_hint {
            if let Some(old_path) = self.get_path_by_docid_in_local_repo(repo_name, doc_id)?
                && old_path != normalized_path
            {
                self.rename_doc_in_local_repo(repo_name, &old_path, normalized_path)?;
            } else if self
                .get_path_by_docid_in_local_repo(repo_name, doc_id)?
                .is_none()
            {
                self.run_on_local_repo(repo_name, |db| {
                    crate::ledger::metadata::set_doc_path(db, doc_id, normalized_path)
                })?;
            }
            return Ok(doc_id);
        }
        if let Some(doc_id) = self.get_docid_in_local_repo(repo_name, normalized_path)? {
            return Ok(doc_id);
        }
        self.create_docid_in_local_repo(repo_name, normalized_path)
    }
}
