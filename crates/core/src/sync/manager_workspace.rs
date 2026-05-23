//! plan_ref:
//!   - 04_storage#projection-contract
//!   - 07_diff_logic#source-control-runtime

use super::{SyncManager, discard_pending, handler, projection_io};
use crate::models::{DocId, LedgerEntry, PeerId};
use crate::protocol::{ScPathTarget, ServerMessage};
use anyhow::Result;

impl SyncManager {
    pub fn discard_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        discard_pending::discard_pending_workdir(self, repo_name, path)
    }

    pub fn discard_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &ScPathTarget,
    ) -> Result<String> {
        discard_pending::discard_pending_target_workdir(self, repo_name, target)
    }

    pub fn persist_doc(&self, doc_id: DocId) -> Result<()> {
        self.persist_doc_in_local_repo(self.repo.local_repo_name(), doc_id)
    }

    pub fn persist_doc_in_local_repo(&self, repo_name: &str, doc_id: DocId) -> Result<()> {
        projection_io::persist_doc(self, repo_name, doc_id).inspect_err(|_| {
            self.mark_projection_writeback_fault(repo_name);
        })
    }

    pub fn remove_projection_path_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        projection_io::remove_projection_path(self, repo_name, path).inspect_err(|_| {
            self.mark_projection_writeback_fault(repo_name);
        })
    }

    /// 应用操作并选择性持久化到 Vault
    pub fn apply_local_op(
        &self,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
        persist: bool,
    ) -> Result<(u64, u64)> {
        self.apply_local_op_in_local_repo(
            self.repo.local_repo_name(),
            doc_id,
            peer_id,
            op_entry_builder,
            persist,
        )
    }

    /// Invariant: 追加 Op、重建文档与保存快照必须命中同一 `repo_name`。
    pub fn apply_local_op_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        peer_id: PeerId,
        op_entry_builder: impl FnMut(u64) -> LedgerEntry,
        persist: bool,
    ) -> Result<(u64, u64)> {
        let seqs = self.repo.append_generated_op_in_local_repo(
            repo_name,
            doc_id,
            peer_id,
            op_entry_builder,
        )?;

        if persist && let Err(e) = self.persist_doc_in_local_repo(repo_name, doc_id) {
            tracing::error!(
                "SyncManager: Failed to persist doc {} after op: {:?}",
                doc_id,
                e
            );
            return Err(e);
        }

        Ok(seqs)
    }

    pub fn handle_fs_event(
        &self,
        repo_name: &str,
        repo_id: crate::models::RepoId,
        repo_path: &str,
    ) -> Result<Vec<ServerMessage>> {
        if self.is_projection_degraded(&repo_name) {
            tracing::warn!(
                repo_name = %repo_name,
                "Ignoring filesystem event for degraded local repo"
            );
            return Ok(vec![]);
        }
        if repo_path.is_empty() {
            return Ok(vec![]);
        }
        let handler = handler::FsEventHandler::new(&self.repo, &self.vfs, &repo_name, repo_id);
        handler.handle_event(repo_path)
    }

    /// Invariant: 仅忽略近期由 SyncManager 自己写回、且内容哈希完全一致的事件。
    pub fn should_ignore_fs_event(&self, repo_name: &str, repo_path: &str) -> bool {
        self.repo
            .should_ignore_workspace_event(repo_name, repo_path)
    }
}
