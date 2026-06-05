//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use super::model::{EditorTabKey, diff_tab_key};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::models::DocId;

pub(crate) fn scope_changed(
    previous: &(Option<String>, u64),
    next: &(Option<String>, u64),
) -> bool {
    previous != next
}

pub(crate) fn should_clear_diff_on_document_change(
    previous: Option<DocId>,
    next: Option<DocId>,
    diff_open: bool,
) -> bool {
    diff_open && previous != next
}

pub(crate) fn active_editor_tab_key(
    diff_content: Option<&DiffSessionWire>,
    current_editor_doc: Option<DocId>,
) -> Option<EditorTabKey> {
    diff_content
        .map(|session| EditorTabKey::Diff(diff_tab_key(session)))
        .or_else(|| current_editor_doc.map(EditorTabKey::Document))
}
