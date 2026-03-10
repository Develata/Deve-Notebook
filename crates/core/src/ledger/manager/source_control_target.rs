use crate::ledger::RepoManager;
use crate::protocol::ScPathTarget;
use anyhow::Result;

use super::source_control_target_lookup::{self, ScTargetScope};

impl RepoManager {
    pub fn stage_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        let path = source_control_target_lookup::resolve_path(
            self,
            repo_name,
            ScTargetScope::Pending,
            target,
        )?;
        self.stage_pending_in_local_repo(repo_name, &path)
    }

    pub fn discard_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        let path = source_control_target_lookup::resolve_path(
            self,
            repo_name,
            ScTargetScope::Pending,
            target,
        )?;
        self.discard_pending_in_local_repo(repo_name, &path)
    }

    pub fn unstage_file_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<()> {
        let path = source_control_target_lookup::resolve_path(
            self,
            repo_name,
            ScTargetScope::Staged,
            target,
        )?;
        self.unstage_file_in_local_repo(repo_name, &path)
    }

    pub fn diff_doc_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<String> {
        let path = source_control_target_lookup::resolve_path(
            self,
            repo_name,
            ScTargetScope::Changes,
            target,
        )?;
        self.diff_doc_path_in_local_repo(repo_name, &path)
    }
}
