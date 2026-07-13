//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 10_rendering#large-document-runtime
//!
use crate::runtime::source_control_client::diff_session::DiffSessionWire;
use deve_core::models::DocId;
use deve_core::source_control::diff_projection::{DiffCellKind, DiffProjection};
use deve_core::source_control::line_diff::ChangeRange;
use leptos::prelude::*;
use std::sync::Arc;

pub(in crate::hooks::use_core) fn apply_doc_diff(
    request_id: Option<&str>,
    doc_id: Option<DocId>,
    path: &str,
    projection: &Arc<DiffProjection>,
    cache_key: Option<String>,
    set_diff: WriteSignal<Option<DiffSessionWire>>,
) {
    leptos::logging::log!("收到 Diff: {}", path);
    set_diff.update(|current| {
        if let Some(session) = current
            && session.matches_pending_request(request_id)
        {
            session.install_document_projection(path.to_string(), doc_id, projection.clone());
            if let Some(cache_key) = cache_key {
                session.cache_key = Some(cache_key);
            }
        }
    });
    let ranges = projection_gutter_ranges(projection);
    #[cfg(target_arch = "wasm32")]
    if let Ok(json) = serde_json::to_string(&ranges) {
        crate::editor::ffi::update_gutter_diff(&json);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = ranges;
}

fn projection_gutter_ranges(projection: &DiffProjection) -> Vec<ChangeRange> {
    let mut ranges = Vec::new();
    let mut insertion_line = 1u32;
    for row in &projection.rows {
        if let Some(line) = row.right.line_number {
            insertion_line = line;
        }
        match (row.left.kind, row.right.kind) {
            (DiffCellKind::Delete, DiffCellKind::Add) => {
                push_range(
                    &mut ranges,
                    "modified",
                    row.right.line_number.unwrap_or(insertion_line),
                );
            }
            (_, DiffCellKind::Add) => {
                push_range(
                    &mut ranges,
                    "added",
                    row.right.line_number.unwrap_or(insertion_line),
                );
            }
            (DiffCellKind::Delete, _) => {
                push_range(&mut ranges, "deleted", insertion_line);
            }
            _ => {}
        }
        if row.right.line_number.is_some() {
            insertion_line = insertion_line.saturating_add(1);
        }
    }
    ranges
}

fn push_range(ranges: &mut Vec<ChangeRange>, kind: &str, line: u32) {
    if let Some(last) = ranges.last_mut()
        && last.kind == kind
        && last.end_line.saturating_add(1) >= line
    {
        last.end_line = last.end_line.max(line);
        return;
    }
    ranges.push(ChangeRange {
        kind: kind.to_string(),
        start_line: line,
        end_line: line,
    });
}

#[cfg(test)]
mod tests {
    use super::projection_gutter_ranges;
    use deve_core::source_control::diff_projection::compute_diff_projection;

    #[test]
    fn gutter_ranges_are_projected_without_recomputing_diff() {
        let projection = compute_diff_projection("a\nold".into(), "a\nnew\nextra".into()).unwrap();
        let ranges = projection_gutter_ranges(&projection);
        assert!(ranges.iter().any(|range| range.kind == "modified"));
        assert!(ranges.iter().any(|range| range.kind == "added"));
    }
}
