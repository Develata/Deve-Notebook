use crate::server::handlers::source_control::present;
use deve_core::protocol::{ScPathTarget, ServerError, ServerErrorCode};
use deve_core::source_control::ChangeEntry;
use deve_core::utils::path::to_forward_slash;
use std::collections::HashSet;

pub fn related_targets(entries: &[ChangeEntry], target: &ScPathTarget) -> Vec<ScPathTarget> {
    present::expand_related_targets(entries, target)
}

pub fn resolve_target(
    entries: &[ChangeEntry],
    target: &ScPathTarget,
) -> super::ScResult<ScPathTarget> {
    let Some(path) = present::resolve_target_path(entries, target) else {
        return Err(ServerError::with_detail(
            ServerErrorCode::ScConflictTargetMissing,
            format!(
                "Source control target not found in current change set: {}",
                target.path
            ),
        ));
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
            && target
                .doc_id
                .map(|doc_id| entry.doc_id == Some(doc_id))
                .unwrap_or(true)
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_target;
    use deve_core::models::DocId;
    use deve_core::protocol::{ScPathTarget, ServerErrorCode};
    use deve_core::source_control::{ChangeEntry, ChangeStatus};

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
    fn rejects_path_only_tracked_rename_successor() {
        let doc_id = DocId(uuid::Uuid::nil());
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
        assert_eq!(err.code, ServerErrorCode::ScConflictTargetMissing);
    }
}
