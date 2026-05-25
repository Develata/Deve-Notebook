//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 03_storage#projection-contract
//!   - 04_repository#tree-projection-contract
//!   - 03_storage#internal-path-normalization
//!
//! # Source Control 工作区 Diff 输入
//!
//! Invariant: 外部 Working Directory diff 的左侧永远来自当前 Ledger 投影，右侧来自磁盘文件。

use crate::ledger::RepoManager;
use crate::models::DocId;
use crate::protocol::ScPathTarget;
use crate::utils::path::to_forward_slash;
use anyhow::Result;

use super::source_control_target_lookup;
use super::source_control_workdir_helpers::{rebuild_doc_projection, workspace_path_exists};

impl RepoManager {
    pub fn workdir_diff_inputs_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<(String, String)> {
        let normalized = to_forward_slash(path);
        let old_doc_id = self.resolve_workdir_doc_id_in_local_repo(repo_name, &normalized)?;
        self.workdir_diff_inputs_for_resolved_target(repo_name, &normalized, old_doc_id)
    }

    pub fn workdir_diff_inputs_for_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<(String, String, String)> {
        let (_, path, old_content, new_content) =
            self.workdir_diff_payload_for_target_in_local_repo(repo_name, target)?;
        Ok((path, old_content, new_content))
    }

    pub fn workdir_diff_payload_for_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<(Option<DocId>, String, String, String)> {
        let path = source_control_target_lookup::resolve_change_path(self, repo_name, target)?;
        let doc_id = match target.doc_id {
            Some(doc_id) => Some(doc_id),
            None => self.resolve_workdir_doc_id_in_local_repo(repo_name, &path)?,
        };
        let (old_content, new_content) =
            self.workdir_diff_inputs_for_resolved_target(repo_name, &path, doc_id)?;
        Ok((doc_id, path, old_content, new_content))
    }

    fn workdir_diff_inputs_for_resolved_target(
        &self,
        repo_name: &str,
        path: &str,
        doc_id: Option<DocId>,
    ) -> Result<(String, String)> {
        let old_content = match doc_id {
            Some(doc_id) => rebuild_doc_projection(self, repo_name, doc_id)?,
            None => String::new(),
        };
        let file_path = self.local_repo_workspace_path(repo_name, path)?;
        let workspace_exists = workspace_path_exists(
            &file_path,
            &format!(
                "Failed to stat workspace path while reading workdir diff {}",
                path
            ),
        )?;
        let new_content = if workspace_exists {
            std::fs::read_to_string(file_path)?
        } else {
            String::new()
        };
        if doc_id.is_none() && !workspace_exists {
            anyhow::bail!("Doc not found: {}", path);
        }
        Ok((old_content, new_content))
    }
}
