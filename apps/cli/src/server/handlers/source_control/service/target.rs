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
mod tests {
    use super::{related_targets, resolve_target, target_exists};
    use deve_core::models::DocId;
    use deve_core::protocol::{ScPathTarget, ServerErrorCode};
    use deve_core::source_control::{ChangeEntry, ChangeStatus};
    use uuid::Uuid;

    #[test]
    fn rejects_unresolved_path_only_target() {
        let err = resolve_target(&[], &ScPathTarget::from_path("notes/missing.md"))
            .expect_err("missing target must fail closed");
        assert_eq!(err.code, ServerErrorCode::ScConflictTargetMissing);
    }

    #[test]
    fn accepts_path_only_rename_successor_when_entry_exists() {
        let entries = vec![
            ChangeEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: None,
                status: ChangeStatus::Deleted,
                has_conflict: false,
            },
            ChangeEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: None,
                status: ChangeStatus::Added,
                has_conflict: false,
            },
        ];
        assert_eq!(
            resolve_target(&entries, &ScPathTarget::from_path("notes/old.md"))
                .expect("rename successor should resolve"),
            ScPathTarget::from_path("notes/new.md")
        );
    }

    #[test]
    fn rejects_path_only_tracked_rename_successor_as_conflict() {
        let doc_id = DocId(Uuid::nil());
        let entries = vec![
            ChangeEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                status: ChangeStatus::Deleted,
                has_conflict: false,
            },
            ChangeEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: Some(doc_id),
                status: ChangeStatus::Added,
                has_conflict: false,
            },
        ];
        let err = resolve_target(&entries, &ScPathTarget::from_path("notes/old.md"))
            .expect_err("tracked path-only target must fail closed");
        assert_eq!(err.code, ServerErrorCode::StorageConflict);
    }

    #[test]
    fn rejects_path_reuse_conflict_as_storage_conflict() {
        let entries = vec![
            ChangeEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: None,
                status: ChangeStatus::Added,
                has_conflict: false,
            },
            ChangeEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: None,
                status: ChangeStatus::Added,
                has_conflict: false,
            },
        ];
        let err = resolve_target(&entries, &ScPathTarget::from_path("notes/old.md"))
            .expect_err("ambiguous path-only target must fail closed");
        assert_eq!(err.code, ServerErrorCode::StorageConflict);
    }

    #[test]
    fn target_exists_rejects_docless_match_against_tracked_entry() {
        let entries = vec![ChangeEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: Some(DocId(Uuid::nil())),
            status: ChangeStatus::Modified,
            has_conflict: false,
        }];
        assert!(!target_exists(
            &entries,
            &ScPathTarget::from_path("notes/a.md")
        ));
    }

    #[test]
    fn target_exists_accepts_docless_match_for_untracked_entry() {
        let entries = vec![ChangeEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            status: ChangeStatus::Added,
            has_conflict: false,
        }];
        assert!(target_exists(
            &entries,
            &ScPathTarget::from_path("notes/a.md")
        ));
    }

    #[test]
    fn related_targets_rejects_tracked_path_only_ambiguity_as_conflict() {
        let doc_id = DocId(Uuid::nil());
        let entries = vec![
            ChangeEntry {
                path: "notes/old.md".into(),
                renamed_from: None,
                doc_id: Some(doc_id),
                status: ChangeStatus::Deleted,
                has_conflict: false,
            },
            ChangeEntry {
                path: "notes/new.md".into(),
                renamed_from: Some("notes/old.md".into()),
                doc_id: Some(doc_id),
                status: ChangeStatus::Added,
                has_conflict: false,
            },
        ];
        let err = related_targets(&entries, &ScPathTarget::from_path("notes/old.md"))
            .expect_err("tracked path-only ambiguity must fail closed");
        assert_eq!(err.code, ServerErrorCode::StorageConflict);
    }
}
