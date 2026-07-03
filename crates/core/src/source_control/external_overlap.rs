//! External Changes overlap helpers.
//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 05_diff_logic#authority-diff-core
//!
//! Keep overlap identity rules in core so UI surfaces consume typed
//! `has_conflict` state instead of re-deriving ledger/source-control authority.

use crate::models::DocId;
use crate::source_control::ChangeEntry;
use crate::utils::path::to_forward_slash;

pub(crate) fn mark_entries_with_confirmed_overlap(
    entries: &mut [ChangeEntry],
    confirmed: &[ChangeEntry],
) {
    for entry in entries {
        if fields_overlap_any_confirmed(
            entry.doc_id,
            &entry.path,
            entry.renamed_from.as_deref(),
            confirmed,
        ) {
            entry.has_conflict = true;
        }
    }
}

pub(crate) fn fields_overlap_any_confirmed(
    doc_id: Option<DocId>,
    path: &str,
    renamed_from: Option<&str>,
    confirmed: &[ChangeEntry],
) -> bool {
    confirmed
        .iter()
        .any(|entry| fields_overlap_confirmed(doc_id, path, renamed_from, entry))
}

pub(crate) fn fields_overlap_confirmed(
    doc_id: Option<DocId>,
    path: &str,
    renamed_from: Option<&str>,
    confirmed: &ChangeEntry,
) -> bool {
    if matches!(
        (doc_id, confirmed.doc_id),
        (Some(left_doc), Some(right_doc)) if left_doc == right_doc
    ) {
        return true;
    }

    let path = to_forward_slash(path);
    let confirmed_path = to_forward_slash(&confirmed.path);
    let renamed_from = renamed_from.map(to_forward_slash);
    let confirmed_renamed_from = confirmed.renamed_from.as_deref().map(to_forward_slash);

    path == confirmed_path
        || confirmed_renamed_from.as_deref() == Some(path.as_str())
        || renamed_from.as_deref() == Some(confirmed_path.as_str())
}
