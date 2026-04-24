//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use super::{expand_related_paths, resolve_target_path_strict};
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeEntry;

pub(crate) fn resolve_target_path(
    entries: &[ChangeEntry],
    target: &ScPathTarget,
) -> Option<String> {
    resolve_target_path_strict(entries, target).ok().flatten()
}

pub(crate) fn expand_related_targets(
    entries: &[ChangeEntry],
    target: &ScPathTarget,
) -> Vec<ScPathTarget> {
    let resolved = resolve_target(entries, target);
    if resolve_target_path(entries, &resolved).is_none() {
        return vec![];
    }
    let mut targets = vec![];
    for path in expand_related_paths(entries, &resolved.path) {
        let doc_id = entries
            .iter()
            .find(|entry| deve_core::utils::path::to_forward_slash(&entry.path) == path)
            .and_then(|entry| entry.doc_id)
            .or(resolved.doc_id);
        let candidate = ScPathTarget { path, doc_id };
        if !targets.contains(&candidate) {
            targets.push(candidate);
        }
    }
    targets
}

fn resolve_target(entries: &[ChangeEntry], target: &ScPathTarget) -> ScPathTarget {
    let Some(path) = resolve_target_path(entries, target) else {
        return target.clone();
    };
    let doc_id = target.doc_id.or_else(|| {
        entries
            .iter()
            .find(|entry| deve_core::utils::path::to_forward_slash(&entry.path) == path)
            .and_then(|entry| entry.doc_id)
    });
    ScPathTarget { path, doc_id }
}
