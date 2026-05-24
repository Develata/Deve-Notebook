//! plan_ref:
//!   - 17_plugins#plugin-runtime-boundary
//!
use crate::ledger::reconcile;
use crate::models::DocId;
use crate::plugin::manifest::Capability;
use crate::state;
use anyhow::Result;
use rhai::{Engine, EvalAltResult};
use std::path::Path;
use std::sync::Arc;

use super::path_guard::{is_ledger_managed_write_target, managed_note_target_parts};

pub fn register_note_api(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_read = caps.clone();
    let caps_write = caps;
    engine.register_fn(
        "note_read",
        move |path: &str| -> Result<String, Box<EvalAltResult>> {
            let target = Path::new(path);
            if !caps_read.check_read(target) {
                return Err(format!(
                    "Permission denied: read access to '{}' is not allowed by manifest.",
                    path
                )
                .into());
            }
            read_managed_note(target).map_err(|e| e.to_string().into())
        },
    );
    engine.register_fn(
        "note_write",
        move |path: &str, content: &str| -> Result<(), Box<EvalAltResult>> {
            let target = Path::new(path);
            if !caps_write.check_write(target) {
                return Err(format!(
                    "Permission denied: write access to '{}' is not allowed by manifest.",
                    path
                )
                .into());
            }
            write_managed_note(target, content).map_err(|e| e.to_string().into())
        },
    );
}

fn read_managed_note(path: &Path) -> Result<String> {
    let doc_id = resolve_managed_note_target(path)?;
    let repo_name = managed_target_parts(path)?.0;
    let repo = super::repo_manager()?;
    let ops = repo.get_local_ops_in_local_repo(&repo_name, doc_id)?;
    let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
    Ok(state::reconstruct_content(&entries))
}

fn write_managed_note(path: &Path, content: &str) -> Result<()> {
    let (repo_name, _repo_path) = managed_target_parts(path)?;
    let repo = super::repo_manager()?;
    let sync = super::sync_manager()?;
    let doc_id = ensure_managed_note_target(path)?;
    let ops = repo.get_local_ops_in_local_repo(&repo_name, doc_id)?;
    let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
    let patch = reconcile::compute_reconcile_patch(&entries, content)?;
    if !patch.is_empty() {
        reconcile::append_patch_in_local_repo(repo.as_ref(), &repo_name, doc_id, "plugin", &patch)?;
    }
    sync.persist_doc_in_local_repo(&repo_name, doc_id)
}

fn ensure_managed_note_target(path: &Path) -> Result<DocId> {
    let (repo_name, repo_path) = managed_target_parts(path)?;
    let (doc_id, _ops) = super::repo_manager()?
        .apply_file_structure_in_local_repo(&repo_name, &repo_path, None, "plugin")?;
    Ok(doc_id)
}

fn resolve_managed_note_target(path: &Path) -> Result<DocId> {
    let (repo_name, repo_path) = managed_target_parts(path)?;
    super::repo_manager()?
        .get_tracked_docid_in_local_repo(&repo_name, &repo_path)?
        .ok_or_else(|| anyhow::anyhow!("managed note not found: {}", repo_path))
}

fn managed_target_parts(path: &Path) -> Result<(String, String)> {
    if !is_ledger_managed_write_target(path).map_err(anyhow::Error::msg)? {
        anyhow::bail!("note API only supports ledger-managed markdown targets");
    }
    managed_note_target_parts(path)
        .map_err(anyhow::Error::msg)?
        .ok_or_else(|| anyhow::anyhow!("note API requires a managed markdown path"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_managed_targets() {
        let err = write_managed_note(Path::new("tmp/out.txt"), "x").expect_err("must fail");
        assert!(err.to_string().contains("ledger-managed markdown"));
        let err = read_managed_note(Path::new("tmp/out.txt")).expect_err("must fail");
        assert!(err.to_string().contains("ledger-managed markdown"));
    }
}
