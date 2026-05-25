//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!   - 03_storage#internal-path-normalization
//!
use crate::ledger::RepoManager;
use crate::protocol::ScPathTarget;
use crate::source_control::{ChangeEntry, ChangeStatus};
use crate::utils::path::to_forward_slash;
use anyhow::{Result, anyhow};

impl RepoManager {
    /// 将旧的 path-only Source Control 入口提升为 tracked target。
    ///
    /// Invariants:
    /// - 若当前 path 已被 node projection 跟踪，则必须补上 `doc_id`。
    /// - 若 path 不是当前 tracked projection，只保留规范化 path，不猜测旧 mapping。
    pub(crate) fn tracked_target_for_path_in_local_repo(
        &self,
        repo_name: &str,
        path: &str,
    ) -> Result<ScPathTarget> {
        let path = to_forward_slash(path);
        let changes = self.list_changes_in_local_repo(repo_name)?;
        let doc_id = if has_legacy_docless_exact_delete(&changes, &path) {
            None
        } else {
            match self.get_tracked_docid_in_local_repo(repo_name, &path)? {
                Some(doc_id) => Some(doc_id),
                None => tracked_doc_id_from_changes(&changes, &path)?,
            }
        };
        Ok(ScPathTarget { doc_id, path })
    }
}

/// Legacy exception: old scanner/repair paths may produce an exact delete row
/// without `doc_id`. Keep it path-only until commit, where delete planning must
/// resolve the current node projection before appending structure facts.
fn has_legacy_docless_exact_delete(entries: &[ChangeEntry], path: &str) -> bool {
    entries.iter().any(|entry| {
        normalized(&entry.path) == path
            && entry.status == ChangeStatus::Deleted
            && entry.doc_id.is_none()
    })
}

fn tracked_doc_id_from_changes(
    entries: &[ChangeEntry],
    path: &str,
) -> Result<Option<crate::models::DocId>> {
    let exact = entries
        .iter()
        .filter(|entry| normalized(&entry.path) == path)
        .collect::<Vec<_>>();
    let renamed = entries
        .iter()
        .filter(|entry| {
            entry.status != ChangeStatus::Deleted
                && entry
                    .renamed_from
                    .as_ref()
                    .is_some_and(|old_path| normalized(old_path) == path)
        })
        .collect::<Vec<_>>();
    if exact
        .iter()
        .chain(renamed.iter())
        .all(|entry| entry.doc_id.is_none())
    {
        return Ok(None);
    }
    let live_exact = exact
        .iter()
        .copied()
        .filter(|entry| entry.status != ChangeStatus::Deleted)
        .collect::<Vec<_>>();
    let deleted_exact = exact
        .iter()
        .any(|entry| entry.status == ChangeStatus::Deleted);
    if live_exact.len() > 1 || renamed.len() > 1 {
        return Err(anyhow!(
            "Ambiguous source control path target: {} matched multiple live tracked entries",
            path
        ));
    }
    if deleted_exact && renamed.len() == 1 {
        return Ok(renamed[0].doc_id);
    }
    if !deleted_exact && !live_exact.is_empty() && !renamed.is_empty() {
        return Err(anyhow!(
            "Ambiguous source control path target: {} matched reused path and rename successor",
            path
        ));
    }
    if let Some(entry) = live_exact.into_iter().next() {
        return Ok(entry.doc_id);
    }
    if let Some(entry) = renamed.into_iter().next() {
        return Ok(entry.doc_id);
    }
    Ok((exact.len() == 1).then_some(exact[0].doc_id).flatten())
}

fn normalized(path: &str) -> String {
    to_forward_slash(path)
}

#[cfg(test)]
mod tests;
