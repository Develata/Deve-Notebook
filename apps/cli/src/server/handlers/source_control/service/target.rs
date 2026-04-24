//! plan_ref:
//!   - 07_diff_logic#source-control-runtime

use crate::server::handlers::source_control::present;
use deve_core::protocol::{ScPathTarget, ServerError, ServerErrorCode};
use deve_core::source_control::ChangeEntry;
use deve_core::utils::path::to_forward_slash;
use std::collections::HashSet;

pub fn related_targets(
    entries: &[ChangeEntry],
    target: &ScPathTarget,
) -> super::ScResult<Vec<ScPathTarget>> {
    let path = match present::resolve_target_path_strict(entries, target) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Err(ServerError::with_detail(
                ServerErrorCode::ScConflictTargetMissing,
                format!(
                    "Source control target not found in current change set: {}",
                    target.path
                ),
            ));
        }
        Err(err) => {
            return Err(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                err.to_string(),
            ));
        }
    };
    let resolved = ScPathTarget {
        doc_id: target.doc_id.or_else(|| {
            entries
                .iter()
                .find(|entry| to_forward_slash(&entry.path) == path)
                .and_then(|entry| entry.doc_id)
        }),
        path,
    };
    let mut targets = vec![];
    for path in present::expand_related_paths(entries, &resolved.path) {
        let doc_id = entries
            .iter()
            .find(|entry| to_forward_slash(&entry.path) == path)
            .and_then(|entry| entry.doc_id)
            .or(resolved.doc_id);
        let candidate = ScPathTarget { path, doc_id };
        if !targets.contains(&candidate) {
            targets.push(candidate);
        }
    }
    Ok(targets)
}

pub fn resolve_target(
    entries: &[ChangeEntry],
    target: &ScPathTarget,
) -> super::ScResult<ScPathTarget> {
    let path = match present::resolve_target_path_strict(entries, target) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return Err(ServerError::with_detail(
                ServerErrorCode::ScConflictTargetMissing,
                format!(
                    "Source control target not found in current change set: {}",
                    target.path
                ),
            ));
        }
        Err(err) => {
            return Err(ServerError::with_detail(
                ServerErrorCode::StorageConflict,
                err.to_string(),
            ));
        }
    };
    let resolved = ScPathTarget {
        doc_id: target.doc_id.or_else(|| {
            entries
                .iter()
                .find(|entry| to_forward_slash(&entry.path) == path)
                .and_then(|entry| entry.doc_id)
        }),
        path,
    };
    if target_exists(entries, &resolved) {
        return Ok(resolved);
    }
    Err(ServerError::with_detail(
        ServerErrorCode::ScConflictTargetMissing,
        format!(
            "Source control target not found in current change set: {}",
            target.path
        ),
    ))
}

pub fn resolve_targets(
    entries: &[ChangeEntry],
    targets: Vec<ScPathTarget>,
) -> super::ScResult<Vec<ScPathTarget>> {
    let mut seen = HashSet::new();
    let resolved = targets
        .into_iter()
        .map(|target| resolve_target(entries, &target))
        .collect::<super::ScResult<Vec<_>>>()?;
    Ok(resolved
        .into_iter()
        .filter(|target| !target.path.is_empty())
        .filter(|target| seen.insert((target.doc_id, target.path.clone())))
        .collect())
}

fn target_exists(entries: &[ChangeEntry], target: &ScPathTarget) -> bool {
    let target_path = to_forward_slash(&target.path);
    entries.iter().any(|entry| {
        to_forward_slash(&entry.path) == target_path
            && match target.doc_id {
                Some(doc_id) => entry.doc_id == Some(doc_id),
                None => entry.doc_id.is_none(),
            }
    })
}

#[cfg(test)]
#[path = "target_test.rs"]
mod tests;
