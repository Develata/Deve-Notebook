//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-scope-runtime
//!
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::{ChangeEntry, ChangeStatus};
use std::collections::HashSet;

pub(super) fn to_target(entry: &ChangeEntry) -> ScPathTarget {
    ScPathTarget {
        path: normalized(&entry.path),
        doc_id: entry.doc_id,
    }
}

pub(crate) fn can_request_doc_diff(entry: &ChangeEntry) -> bool {
    !(entry.status == ChangeStatus::Deleted && entry.doc_id.is_none())
}

pub(super) fn to_targets(entries: Vec<ChangeEntry>) -> Vec<ScPathTarget> {
    let mut seen = HashSet::new();
    entries
        .into_iter()
        .map(|entry| to_target(&entry))
        .filter(|target| !target.path.is_empty())
        .filter(|target| seen.insert((target.doc_id, target.path.clone())))
        .collect()
}

fn normalized(path: &str) -> String {
    deve_core::utils::path::to_forward_slash(path)
}

#[cfg(test)]
#[path = "callbacks_sc_target_test.rs"]
mod tests;
