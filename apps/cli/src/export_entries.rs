use crate::admin_api::ExportEntry;
use anyhow::Result;
use deve_core::ledger::{RepoManager, node_meta, range};
use deve_core::models::LedgerEntry;

pub fn build(repo: &RepoManager, repo_name: &str) -> Result<Vec<ExportEntry>> {
    let ops = repo.run_on_local_repo(repo_name, |db| {
        let max_seq = range::get_max_seq(db)?;
        range::get_ops_in_range(db, 1, max_seq + 1)
    })?;
    let mut entries = Vec::with_capacity(ops.len());
    for (global_seq, entry) in ops {
        entries.push(ExportEntry {
            global_seq,
            current_path: resolve_path(repo, repo_name, &entry)?,
            entry,
        });
    }
    Ok(entries)
}

fn resolve_path(
    repo: &RepoManager,
    repo_name: &str,
    entry: &LedgerEntry,
) -> Result<Option<String>> {
    if let Some(doc_id) = entry.doc_id {
        return repo.get_path_by_docid_in_local_repo(repo_name, doc_id);
    }
    if let Some(node_id) = entry.structure_node_id() {
        return repo.run_on_local_repo(repo_name, |db| {
            Ok(node_meta::get_node_meta(db, node_id)?.map(|meta| meta.path))
        });
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn includes_dir_structure_fact_in_export() {
        let dir = TempDir::new().expect("create tempdir");
        let repo = RepoManager::init(dir.path().join("ledger"), 10, None, None).expect("init repo");
        repo.apply_dir_create_structure_in_local_repo(repo.local_repo_name(), "notes", "test")
            .expect("create dir");

        let entries = build(&repo, repo.local_repo_name()).expect("build export");
        assert!(entries.iter().any(|entry| {
            entry.entry.doc_id.is_none() && entry.current_path.as_deref() == Some("notes")
        }));
    }
}
