use crate::plugin::manifest::Capability;
use crate::sync::reconcile;
use anyhow::Result;
use rhai::{Engine, EvalAltResult};
use std::path::Path;
use std::sync::Arc;

use super::path_guard::{
    is_ledger_managed_write_target, project_relative_path, split_managed_note_target,
};

pub fn register_note_api(engine: &mut Engine, caps: Arc<Capability>) {
    let caps_write = caps;
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

fn write_managed_note(path: &Path, content: &str) -> Result<()> {
    if !is_ledger_managed_write_target(path).map_err(anyhow::Error::msg)? {
        anyhow::bail!("note_write only supports ledger-managed markdown targets");
    }
    let cwd = std::env::current_dir()?;
    let rel = project_relative_path(&cwd, path)
        .ok_or_else(|| anyhow::anyhow!("note_write target is outside project root"))?;
    let (repo_name, repo_path) = split_managed_note_target(&rel)
        .ok_or_else(|| anyhow::anyhow!("note_write requires a managed markdown path"))?;
    let repo = super::repo_manager()?;
    let sync = super::sync_manager()?;
    let doc_id = repo.apply_file_structure_in_local_repo(&repo_name, &repo_path, None, "plugin")?;
    let ops = repo.get_local_ops_in_local_repo(&repo_name, doc_id)?;
    let entries: Vec<_> = ops.into_iter().map(|(_, entry)| entry).collect();
    let patch = reconcile::compute_reconcile_patch(&entries, content)?;
    if !patch.is_empty() {
        reconcile::append_patch_in_local_repo(repo.as_ref(), &repo_name, doc_id, "plugin", &patch)?;
    }
    sync.persist_doc_in_local_repo(&repo_name, doc_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_managed_targets() {
        let err = write_managed_note(Path::new("tmp/out.txt"), "x").expect_err("must fail");
        assert!(err.to_string().contains("ledger-managed markdown"));
    }
}
