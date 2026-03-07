// crates\core\src\sync
pub mod buffer;
#[cfg(not(target_arch = "wasm32"))]
pub mod engine;
#[cfg(not(target_arch = "wasm32"))]
pub mod handler;
#[cfg(not(target_arch = "wasm32"))]
mod materialize;
#[cfg(not(target_arch = "wasm32"))]
mod persist_guard;
pub mod protocol;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod rebuild;
#[cfg(not(target_arch = "wasm32"))]
pub mod reconcile;
#[cfg(not(target_arch = "wasm32"))]
pub mod recovery;
#[cfg(not(target_arch = "wasm32"))]
pub mod repo_scoped;
#[cfg(not(target_arch = "wasm32"))]
pub mod scan;
#[cfg(not(target_arch = "wasm32"))]
pub mod snapshot_policy;
pub mod vector;

#[cfg(not(target_arch = "wasm32"))]
use crate::ledger::RepoManager;
#[cfg(not(target_arch = "wasm32"))]
use crate::models::DocId;
#[cfg(not(target_arch = "wasm32"))]
use crate::models::RepoId;
#[cfg(not(target_arch = "wasm32"))]
use crate::vfs::Vfs;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use persist_guard::PersistGuard;
#[cfg(not(target_arch = "wasm32"))]
use snapshot_policy::SnapshotPolicy;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use tracing::{info, warn};
#[cfg(not(target_arch = "wasm32"))]
pub struct SyncManager {
    repo: Arc<RepoManager>,
    vault_root: PathBuf,
    vfs: Vfs,
    persist_guard: PersistGuard,
}

#[cfg(not(target_arch = "wasm32"))]
impl SyncManager {
    pub fn new(repo: Arc<RepoManager>, vault_root: PathBuf) -> Self {
        // 创建 .notegit/ 目录结构（类似 .git）
        let notegit_dir = vault_root.join(".notegit");
        if let Err(e) = std::fs::create_dir_all(&notegit_dir) {
            warn!("Failed to create .notegit/ directory: {:?}", e);
        }

        let vfs = Vfs::new(&vault_root);
        Self {
            repo,
            vault_root,
            vfs,
            persist_guard: PersistGuard::new(),
        }
    }

    pub fn scan(&self) -> Result<()> {
        scan::scan_vault(&self.repo, &self.vfs, &self.vault_root)
    }

    pub fn reconcile_doc(&self, doc_id: DocId) -> Result<bool> {
        self.reconcile_doc_in_local_repo(self.repo.local_repo_name(), doc_id)
    }

    pub fn reconcile_doc_in_local_repo(&self, repo_name: &str, doc_id: DocId) -> Result<bool> {
        if let Some(path_str) = self
            .repo
            .get_path_by_docid_in_local_repo(repo_name, doc_id)?
        {
            let file_path = self.vault_root.join(&path_str);

            if file_path.exists() {
                let disk_content = std::fs::read_to_string(&file_path)?;
                let ops = self.repo.get_local_ops_in_local_repo(repo_name, doc_id)?;

                let fix_ops = reconcile::compute_reconcile_ops(
                    doc_id,
                    &ops.into_iter().map(|(_, e)| e).collect::<Vec<_>>(),
                    &disk_content,
                )?;

                if !fix_ops.is_empty() {
                    info!(
                        "SyncManager: Applying {} fix ops for {}",
                        fix_ops.len(),
                        path_str
                    );
                    for entry in fix_ops {
                        self.repo.append_local_op_in_local_repo(repo_name, &entry)?;
                    }
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
        if let Some(path_str) = self
            .repo
            .get_path_by_docid_in_local_repo(repo_name, doc_id)?
        {
            let file_path = self.vault_root.join(&path_str);
            let rebuilt = rebuild::rebuild_local_doc_in_repo(&self.repo, repo_name, doc_id)?;

            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&file_path, &rebuilt.content)?;
            self.persist_guard.record(&path_str, &rebuilt.content);
            info!("SyncManager: Persisted doc {} to {:?}", doc_id, file_path);
            let delta = rebuilt.max_seq.saturating_sub(rebuilt.base_seq);
            let policy = SnapshotPolicy::default();
            let doc_len = rebuilt.content.encode_utf16().count();
            if rebuilt.max_seq > 0
                && policy.should_snapshot(doc_len, delta, 0)
                && let Err(e) = self.repo.save_snapshot_in_local_repo(
                    repo_name,
                    doc_id,
                    rebuilt.max_seq,
                    &rebuilt.content,
                )
            {
                warn!(
                    "SyncManager: Failed to save snapshot for {}: {:?}",
                    doc_id, e
                );
            }
        }
        Ok(())
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
        let handler = handler::FsEventHandler::new(&self.repo, &self.vfs, &self.vault_root);
        handler.handle_event(path_str, self)
    }

    /// Invariant: 仅忽略近期由 SyncManager 自己写回、且内容哈希完全一致的事件。
    pub fn should_ignore_fs_event(&self, path_str: &str) -> bool {
        self.persist_guard.should_ignore(&self.vault_root, path_str)
    }

    /// Pre-condition: `repo_name` 必须已解析为真实本地 repo 名称。
    pub fn materialize_local_repo(&self, repo_name: &str) -> Result<()> {
        materialize::materialize_local_repo(
            &self.repo,
            &self.vault_root,
            &self.persist_guard,
            repo_name,
        )
    }

    pub fn local_repo_id(&self) -> Option<RepoId> {
        self.repo
            .get_repo_info()
            .ok()
            .flatten()
            .map(|info| info.uuid)
    }
}
