// crates\core\src\sync
pub mod buffer;
#[cfg(not(target_arch = "wasm32"))]
mod dir_change;
#[cfg(not(target_arch = "wasm32"))]
mod dir_refresh_guard;
#[cfg(not(target_arch = "wasm32"))]
mod discard_pending;
#[cfg(not(target_arch = "wasm32"))]
pub mod engine;
#[cfg(not(target_arch = "wasm32"))]
pub mod handler;
#[cfg(not(target_arch = "wasm32"))]
mod materialize;
#[cfg(not(target_arch = "wasm32"))]
mod pending;
#[cfg(not(target_arch = "wasm32"))]
mod pending_content;
#[cfg(not(target_arch = "wasm32"))]
mod pending_rename;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod persist_guard;
#[cfg(not(target_arch = "wasm32"))]
mod projection_io;
#[cfg(not(target_arch = "wasm32"))]
mod projection_plan;
pub mod protocol;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod rebuild;
#[cfg(not(target_arch = "wasm32"))]
mod rebuild_projection;
#[cfg(not(target_arch = "wasm32"))]
pub mod reconcile;
#[cfg(not(target_arch = "wasm32"))]
pub mod repo_scoped;
#[cfg(not(target_arch = "wasm32"))]
pub mod scan;
#[cfg(not(target_arch = "wasm32"))]
mod scan_file;
#[cfg(not(target_arch = "wasm32"))]
pub mod snapshot_policy;
pub mod vector;

#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::RepoManager;
#[cfg(not(target_arch = "wasm32"))]
use crate::models::DocId;
#[cfg(not(target_arch = "wasm32"))]
use crate::vfs::Vfs;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use dir_refresh_guard::DirRefreshGuard;
#[cfg(not(target_arch = "wasm32"))]
use persist_guard::PersistGuard;
#[cfg(not(target_arch = "wasm32"))]
use snapshot_policy::SnapshotPolicy;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tracing::info;
#[cfg(not(target_arch = "wasm32"))]
pub struct SyncManager {
    repo: Arc<RepoManager>,
    vault_root: PathBuf,
    vfs: Vfs,
    dir_refresh_guard: DirRefreshGuard,
    persist_guard: Arc<PersistGuard>,
}

#[cfg(not(target_arch = "wasm32"))]
impl SyncManager {
    pub fn new(repo: Arc<RepoManager>, vault_root: PathBuf) -> Self {
        let vfs = Vfs::new(&vault_root);
        Self {
            dir_refresh_guard: DirRefreshGuard::new(),
            persist_guard: repo.persist_guard.clone(),
            repo,
            vault_root,
            vfs,
        }
    }

    pub fn scan(&self) -> Result<()> {
        materialize::prepare_local_workspaces(&self.repo, &self.vault_root, &self.persist_guard)?;
        scan::scan_vault(&self.repo, &self.vfs, &self.vault_root)
    }

    pub fn reconcile_doc(&self, doc_id: DocId) -> Result<bool> {
        self.reconcile_doc_in_local_repo(self.repo.local_repo_name(), doc_id)
    }

    pub fn discard_pending_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        discard_pending::discard_pending_workdir(self, repo_name, path)
    }

    pub fn discard_pending_target_in_local_repo(
        &self,
        repo_name: &str,
        target: &crate::protocol::ScPathTarget,
    ) -> Result<String> {
        discard_pending::discard_pending_target_workdir(self, repo_name, target)
    }

    pub fn reconcile_doc_in_local_repo(&self, repo_name: &str, doc_id: DocId) -> Result<bool> {
        if let Some(path_str) = self
            .repo
            .get_file_meta_for_doc_in_local_repo(repo_name, doc_id)?
            .map(|meta| meta.path)
        {
            let file_path = self.repo.local_repo_workspace_path(repo_name, &path_str)?;

            if file_path.exists() {
                let disk_content = std::fs::read_to_string(&file_path)?;
                let ops = self.repo.get_local_ops_in_local_repo(repo_name, doc_id)?;

                let fix_ops = reconcile::compute_reconcile_patch(
                    &ops.into_iter().map(|(_, e)| e).collect::<Vec<_>>(),
                    &disk_content,
                )?;

                if !fix_ops.is_empty() {
                    info!(
                        "SyncManager: Applying {} fix ops for {}",
                        fix_ops.len(),
                        path_str
                    );
                    reconcile::append_patch_in_local_repo(
                        self.repo.as_ref(),
                        repo_name,
                        doc_id,
                        "local_reconcile",
                        &fix_ops,
                    )?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn persist_doc(&self, doc_id: DocId) -> Result<()> {
        self.persist_doc_in_local_repo(self.repo.local_repo_name(), doc_id)
    }
    pub fn persist_doc_in_local_repo(&self, repo_name: &str, doc_id: DocId) -> Result<()> {
        projection_io::persist_doc(self, repo_name, doc_id)
    }

    pub fn remove_projection_path_in_local_repo(&self, repo_name: &str, path: &str) -> Result<()> {
        projection_io::remove_projection_path(self, repo_name, path)
    }

    /// 应用操作并选择性持久化到 Vault
    pub fn apply_local_op(
        &self,
        doc_id: DocId,
        peer_id: crate::models::PeerId,
        op_entry_builder: impl FnMut(u64) -> crate::models::LedgerEntry,
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
        peer_id: crate::models::PeerId,
        op_entry_builder: impl FnMut(u64) -> crate::models::LedgerEntry,
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

    pub fn handle_fs_event(&self, path_str: &str) -> Result<Vec<crate::protocol::ServerMessage>> {
        let Some((repo_name, repo_id, repo_path)) =
            self.repo.resolve_local_workspace_path(path_str)?
        else {
            return Ok(vec![]);
        };
        if repo_path.is_empty() {
            return Ok(vec![]);
        }
        let handler = handler::FsEventHandler::new(&self.repo, &self.vfs, &repo_name, repo_id);
        handler.handle_event(&repo_path)
    }

    /// Invariant: 仅忽略近期由 SyncManager 自己写回、且内容哈希完全一致的事件。
    pub fn should_ignore_fs_event(&self, path_str: &str) -> bool {
        self.repo.should_ignore_workspace_event(path_str)
    }

    /// Pre-condition: `repo_name` 必须已解析为真实本地 repo 名称。
    pub fn materialize_local_repo(&self, repo_name: &str) -> Result<()> {
        materialize::materialize_local_repo(&self.repo, &self.persist_guard, repo_name)
    }

    /// 显式强制重建指定 repo 的 Vault projection。
    pub fn rebuild_projection_local_repo(&self, repo_name: &str) -> Result<()> {
        rebuild_projection::rebuild_local_repo(&self.repo, &self.persist_guard, repo_name)
    }
}
