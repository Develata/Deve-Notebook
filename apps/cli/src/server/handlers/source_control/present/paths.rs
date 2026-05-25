//! plan_ref:
//!   - 05_diff_logic#source-control-runtime

use deve_core::source_control::{ChangeEntry, ChangeStatus};
use std::collections::HashSet;

pub fn collapse_rename_candidates(entries: Vec<ChangeEntry>) -> Vec<ChangeEntry> {
    let hidden: HashSet<(Option<deve_core::models::DocId>, String)> = entries
        .iter()
        .filter_map(|entry| {
            entry.renamed_from.as_ref().map(|old_path| {
                (
                    entry.doc_id,
                    deve_core::utils::path::to_forward_slash(old_path),
                )
            })
        })
        .collect();

    entries
        .into_iter()
        .filter(|entry| {
            !(entry.status == ChangeStatus::Deleted
                && hidden.contains(&(entry.doc_id, normalized(&entry.path))))
        })
        .collect()
}

#[cfg(test)]
pub fn expand_related_paths(entries: &[ChangeEntry], path: &str) -> Vec<String> {
    let path = normalized(path);
    let Some(current) = entries.iter().find(|entry| normalized(&entry.path) == path) else {
        return vec![path];
    };
    let mut paths = vec![path];
    if let Some(old_path) = current.renamed_from.as_ref() {
        paths.push(normalized(old_path));
    } else if current.status == ChangeStatus::Deleted
        && let Some(doc_id) = current.doc_id
        && let Some(added) = entries.iter().find(|entry| {
            entry.status == ChangeStatus::Added
                && entry.doc_id == Some(doc_id)
                && entry.renamed_from.as_deref() == Some(current.path.as_str())
        })
    {
        paths.push(normalized(&added.path));
    }
    paths.sort();
    paths.dedup();
    paths
}

fn normalized(path: &str) -> String {
    deve_core::utils::path::to_forward_slash(path)
}
