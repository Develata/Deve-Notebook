// crates\core\src\sync
pub mod buffer;
#[cfg(not(target_arch = "wasm32"))]
pub mod engine;
#[cfg(not(target_arch = "wasm32"))]
pub mod handler;
pub mod protocol;
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
use tracing::info;

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

            // Reconstruct
            let ops = self.repo.get_local_ops(doc_id)?;
            let content = crate::state::reconstruct_content(
                &ops.into_iter().map(|(_, e)| e).collect::<Vec<_>>(),
            );

            // Write
            std::fs::write(&file_path, content)?;
            info!("SyncManager: Persisted doc {} to {:?}", doc_id, file_path);
        }
        Ok(())
    }

    // --- The Main Logic: Orchestration ---
    pub fn handle_fs_event(&self, path_str: &str) -> Result<Vec<crate::protocol::ServerMessage>> {
        let handler = handler::FsEventHandler::new(&self.repo, &self.vfs, &self.vault_root);
        handler.handle_event(path_str, self)
    }
}
