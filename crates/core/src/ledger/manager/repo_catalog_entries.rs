use crate::ledger::manager::types::RepoManager;
use anyhow::{Context, Result, anyhow};
use std::path::{Path, PathBuf};

pub(crate) fn redb_repo_entries(dir: &Path, context: &str) -> Result<Vec<(PathBuf, String)>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("redb") {
            continue;
        }
        if !entry
            .file_type()
            .with_context(|| {
                format!(
                    "Failed to read repo entry type while {}: {:?}",
                    context, path
                )
            })?
            .is_file()
        {
            return Err(anyhow!(
                "Broken repo entry {:?} while {}: expected file",
                path,
                context
            ));
        }
        let stem = RepoManager::repo_stem_from_path(&path, context)?;
        entries.push((path, stem));
    }
    entries.sort_by(|(left_path, _), (right_path, _)| left_path.cmp(right_path));
    Ok(entries)
}
