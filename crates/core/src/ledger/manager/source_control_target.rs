use crate::ledger::RepoManager;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, ChangeStatus};
use crate::utils::path::to_forward_slash;
use anyhow::Result;

enum ScTargetScope {
    Pending,
    Changes,
}

impl RepoManager {
    pub fn stage_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        let path =
            self.resolve_sc_target_path_in_local_repo(repo_name, ScTargetScope::Pending, target)?;
        self.stage_pending_in_local_repo(repo_name, &path)
    }

    pub fn discard_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        let path =
            self.resolve_sc_target_path_in_local_repo(repo_name, ScTargetScope::Pending, target)?;
        self.discard_pending_in_local_repo(repo_name, &path)
    }

    pub fn unstage_file_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        let path =
            self.resolve_sc_target_path_in_local_repo(repo_name, ScTargetScope::Changes, target)?;
        self.unstage_file_in_local_repo(repo_name, &path)
    }

    pub fn diff_doc_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<String> {
        let path =
            self.resolve_sc_target_path_in_local_repo(repo_name, ScTargetScope::Changes, target)?;
        self.diff_doc_path_in_local_repo(repo_name, &path)
    }

    fn resolve_sc_target_path_in_local_repo(
        &self,
        repo_name: &str,
        scope: ScTargetScope,
        target: &ScPathTarget,
    ) -> Result<String> {
        let path = to_forward_slash(&target.path);
        let entries = match scope {
            ScTargetScope::Pending => self.list_pending_fs_in_local_repo(repo_name)?,
            ScTargetScope::Changes => self.list_changes_in_local_repo(repo_name)?,
        };

        Ok(resolve_path(&entries, &path, target.doc_id)
            .or_else(|| {
                target
                    .doc_id
                    .and_then(|doc_id| {
                        self.get_path_by_docid_in_local_repo(repo_name, doc_id)
                            .ok()
                            .flatten()
                    })
                    .map(|resolved| to_forward_slash(&resolved))
            })
            .unwrap_or(path))
    }
}

fn resolve_path(
    entries: &[ChangeEntry],
    path: &str,
    doc_id: Option<crate::models::DocId>,
) -> Option<String> {
    doc_id
        .and_then(|doc_id| {
            entries
                .iter()
                .find(|entry| {
                    entry.doc_id == Some(doc_id)
                        && to_forward_slash(&entry.path) == path
                        && entry.status != ChangeStatus::Deleted
                })
                .or_else(|| {
                    entries.iter().find(|entry| {
                        entry.doc_id == Some(doc_id) && entry.status != ChangeStatus::Deleted
                    })
                })
                .or_else(|| {
                    entries.iter().find(|entry| {
                        entry.doc_id == Some(doc_id) && to_forward_slash(&entry.path) == path
                    })
                })
                .or_else(|| entries.iter().find(|entry| entry.doc_id == Some(doc_id)))
        })
        .or_else(|| {
            entries
                .iter()
                .find(|entry| to_forward_slash(&entry.path) == path)
        })
        .map(|entry| to_forward_slash(&entry.path))
}
