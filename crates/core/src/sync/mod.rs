// crates\core\src\sync
pub mod buffer;
#[cfg(not(target_arch = "wasm32"))]
pub mod engine;
#[cfg(not(target_arch = "wasm32"))]
pub mod handler;
pub mod protocol;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod rebuild;
#[cfg(not(target_arch = "wasm32"))]
pub mod reconcile;
#[cfg(not(target_arch = "wasm32"))]
pub mod recovery;
#[cfg(not(target_arch = "wasm32"))]
pub mod scan;
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
}

#[cfg(not(target_arch = "wasm32"))]
impl SyncManager {
    pub fn new(repo: Arc<RepoManager>, vault_root: PathBuf) -> Self {
        let vfs = Vfs::new(&vault_root);
        Self {
            repo,
            vault_root,
            vfs,
        }
    }

    pub fn scan(&self) -> Result<()> {
        scan::scan_vault(&self.repo, &self.vfs, &self.vault_root)
    }

    pub fn reconcile_doc(&self, doc_id: DocId) -> Result<bool> {
        if let Some(path_str) = self.repo.get_path_by_docid(doc_id)? {
            let file_path = self.vault_root.join(&path_str);

            if file_path.exists() {
                let disk_content = std::fs::read_to_string(&file_path)?;
                let ops = self.repo.get_local_ops(doc_id)?;

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
                        self.repo.append_local_op(&entry)?;
                    }
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn persist_doc(&self, doc_id: DocId) -> Result<()> {
        if let Some(path_str) = self.repo.get_path_by_docid(doc_id)? {
            let file_path = self.vault_root.join(&path_str);
            let rebuilt = rebuild::rebuild_local_doc(&self.repo, doc_id)?;

            // Write
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&file_path, &rebuilt.content)?;
            info!("SyncManager: Persisted doc {} to {:?}", doc_id, file_path);

            // Snapshot throttle: only update snapshot when it is sufficiently stale.
            const SNAPSHOT_INTERVAL: u64 = 64;
            let delta = rebuilt.max_seq.saturating_sub(rebuilt.base_seq);
            if rebuilt.max_seq > 0 && delta >= SNAPSHOT_INTERVAL {
                if let Err(e) = self
                    .repo
                    .save_snapshot(doc_id, rebuilt.max_seq, &rebuilt.content)
                {
                    warn!(
                        "SyncManager: Failed to save snapshot for {}: {:?}",
                        doc_id, e
                    );
                }
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
        // 1. Append Op to Ledger
        let seqs = crate::ledger::ops::append_generated_op(
            &self.repo.local_db,
            doc_id,
            peer_id,
            op_entry_builder,
        )?;

        // 2. Optional Persist
        if persist {
            if let Err(e) = self.persist_doc(doc_id) {
                tracing::error!(
                    "SyncManager: Failed to persist doc {} after op: {:?}",
                    doc_id,
                    e
                );
                // We don't rollback the op, but we log headers error.
                // In a perfect world we might want transactionality across FS and DB, but that's hard.
                return Err(e);
            }
        }

        Ok(seqs)
    }

    // --- The Main Logic: Orchestration ---
    pub fn handle_fs_event(&self, path_str: &str) -> Result<Vec<crate::protocol::ServerMessage>> {
        let handler = handler::FsEventHandler::new(&self.repo, &self.vfs, &self.vault_root);
        handler.handle_event(path_str, self)
    }
}
