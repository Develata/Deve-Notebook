//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 10_rendering#large-document-runtime
//!
use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::models::DocId;
use leptos::prelude::*;

pub(in crate::hooks::use_core) fn apply_doc_diff(
    doc_id: Option<DocId>,
    path: &str,
    old_content: &str,
    new_content: &str,
    set_diff: WriteSignal<Option<DiffSessionWire>>,
) {
    leptos::logging::log!("收到 Diff: {}", path);
    set_diff.set(Some(
        DiffSessionWire::new(
            path.to_string(),
            old_content.to_string(),
            new_content.to_string(),
        )
        .with_doc_id(doc_id),
    ));
    let ranges =
        deve_core::source_control::line_diff::compute_line_ranges(old_content, new_content);
    #[cfg(target_arch = "wasm32")]
    if let Ok(json) = serde_json::to_string(&ranges) {
        crate::editor::ffi::update_gutter_diff(&json);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = ranges;
}
