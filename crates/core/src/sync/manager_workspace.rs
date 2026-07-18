//! plan_ref:
//!   - 03_storage/projection#projection-contract
//!   - 03_storage/watcher#watcher-contract
//!   - 05_diff_logic#source-control-runtime

use super::{SyncManager, discard_pending, handler, projection_io};
use crate::models::{DocId, LedgerEntry, PeerId};
use crate::protocol::ScPathTarget;
use crate::sync::watcher::WatcherRefresh;
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
        let target_path = self
            .repo
            .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)
            .ok()
            .flatten()
            .map(|meta| meta.path);
        match projection_io::persist_doc(self, repo_name, doc_id) {
            Ok(()) => Ok(()),
            Err(err) => {
                if let Err(fault_error) = self.mark_projection_writeback_fault_for_doc(
                    repo_name,
                    doc_id,
                    target_path.as_deref(),
                    &err,
                ) {
                    return Err(err.context(format!(
                        "failed to persist Projection Fault evidence: {fault_error}"
                    )));
                }
                Err(err)
            }
        }
    }

    /// Writes content prepared from a stable preflight without rebuilding the
    /// document while the server repository permit is held.
    pub fn persist_prepared_doc_content_in_local_repo(
        &self,
        repo_name: &str,
        doc_id: DocId,
        content: &str,
    ) -> Result<()> {
        let target_path = self
            .repo
            .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
            .map(|meta| meta.path)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Tracked document projection missing for {} in repo {} while persisting prepared projection",
                    doc_id,
                    repo_name
                )
            })?;
        match projection_io::persist_prepared_doc_content(
            self,
            repo_name,
            doc_id,
            &target_path,
            content,
        ) {
            Ok(()) => Ok(()),
            Err(error) => {
                if let Err(fault_error) = self.mark_projection_writeback_fault_for_doc(
                    repo_name,
                    doc_id,
                    Some(&target_path),
                    &error,
                ) {
                    return Err(error.context(format!(
                        "failed to persist Projection Fault evidence: {fault_error}"
                    )));
                }
                Err(error)
            }
        }
    }

    pub fn remove_projection_path_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        match projection_io::remove_projection_path(self, repo_name, path) {
            Ok(()) => Ok(()),
            Err(err) => {
                if let Err(fault_error) =
                    self.mark_projection_writeback_fault_for_path(repo_name, path, &err)
                {
                    return Err(err.context(format!(
                        "failed to persist Projection Fault evidence: {fault_error}"
                    )));
                }
                Err(err)
            }
        }
    }

    /// 应用操作并选择性持久化到 Projection Workspace。
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
    ) -> Result<Vec<WatcherRefresh>> {
        if self.is_projection_degraded(repo_name) {
            tracing::warn!(
                repo_name = %repo_name,
                "Ignoring filesystem event for degraded local repo"
            );
            return Ok(vec![]);
        }
        if repo_path.is_empty() {
            return Ok(vec![]);
        }
        let handler = handler::FsEventHandler::new(&self.repo, &self.vfs, repo_name, repo_id);
        handler.handle_event(repo_path)
    }

    /// Invariant: 仅忽略近期由 SyncManager 自己写回、且内容哈希完全一致的事件。
    pub fn should_ignore_fs_event(&self, repo_name: &str, repo_path: &str) -> bool {
        self.repo
            .should_ignore_workspace_event(repo_name, repo_path)
    }
}
