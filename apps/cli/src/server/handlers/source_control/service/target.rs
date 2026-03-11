use crate::server::handlers::source_control::present;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeEntry;
use std::collections::HashSet;

pub fn related_targets(entries: &[ChangeEntry], target: &ScPathTarget) -> Vec<ScPathTarget> {
    present::expand_related_targets(entries, target)
}

pub fn resolve_target(entries: &[ChangeEntry], target: &ScPathTarget) -> ScPathTarget {
    present::resolve_target(entries, target)
}

pub fn resolve_path(entries: &[ChangeEntry], target: &ScPathTarget) -> String {
    resolve_target(entries, target).path
}

pub fn resolve_targets(entries: &[ChangeEntry], targets: Vec<ScPathTarget>) -> Vec<ScPathTarget> {
    let mut seen = HashSet::new();
    targets
        .into_iter()
        .map(|target| resolve_target(entries, &target))
        .filter(|target| !target.path.is_empty())
        .filter(|target| seen.insert((target.doc_id, target.path.clone())))
        .collect()
}
