//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use super::model::{EditorDiffTab, EditorDocumentTab};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::models::DocId;

pub(crate) fn upsert_document_tab(tabs: &mut Vec<EditorDocumentTab>, tab: EditorDocumentTab) {
    if let Some(existing) = tabs
        .iter_mut()
        .find(|existing| existing.doc_id == tab.doc_id)
    {
        *existing = tab;
        return;
    }
    tabs.push(tab);
}

pub(crate) fn upsert_diff_tab(tabs: &mut Vec<EditorDiffTab>, tab: EditorDiffTab) {
    if let Some(existing) = tabs.iter_mut().find(|existing| existing.key == tab.key) {
        *existing = tab;
        return;
    }
    tabs.push(tab);
}

pub(crate) fn remove_document_tab(
    tabs: &mut Vec<EditorDocumentTab>,
    doc_id: DocId,
) -> Option<DocId> {
    let index = tabs.iter().position(|tab| tab.doc_id == doc_id)?;
    tabs.remove(index);
    tabs.get(index)
        .or_else(|| index.checked_sub(1).and_then(|prev| tabs.get(prev)))
        .map(|tab| tab.doc_id)
}

pub(crate) fn remove_diff_tab(tabs: &mut Vec<EditorDiffTab>, key: &str) -> Option<DiffSessionWire> {
    let index = tabs.iter().position(|tab| tab.key == key)?;
    tabs.remove(index);
    tabs.get(index)
        .or_else(|| index.checked_sub(1).and_then(|prev| tabs.get(prev)))
        .map(|tab| tab.session.clone())
}
