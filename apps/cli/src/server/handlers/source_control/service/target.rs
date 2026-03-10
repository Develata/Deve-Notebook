use crate::server::handlers::source_control::present;
use deve_core::protocol::ScPathTarget;
use deve_core::source_control::ChangeEntry;
use std::collections::HashSet;

pub fn related_paths(entries: &[ChangeEntry], target: &ScPathTarget) -> Vec<String> {
    present::expand_related_target_paths(entries, target)
}

pub fn resolve_path(entries: &[ChangeEntry], target: &ScPathTarget) -> String {
    present::resolve_target_path(entries, target)
}

pub fn resolve_paths(entries: &[ChangeEntry], targets: Vec<ScPathTarget>) -> Vec<String> {
    let mut seen = HashSet::new();
    targets
        .into_iter()
        .map(|target| resolve_path(entries, &target))
        .filter(|path| !path.is_empty())
        .filter(|path| seen.insert(path.clone()))
        .collect()
}
