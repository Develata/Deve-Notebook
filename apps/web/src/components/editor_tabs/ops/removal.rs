//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use super::super::model::{EditorDiffTab, EditorDocumentTab, EditorTabKey};
use super::remove_visible_tab_order;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::models::DocId;

#[cfg(test)]
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

#[cfg(test)]
pub(crate) fn remove_diff_tab(tabs: &mut Vec<EditorDiffTab>, key: &str) -> Option<DiffSessionWire> {
    let index = tabs.iter().position(|tab| tab.key == key)?;
    tabs.remove(index);
    tabs.get(index)
        .or_else(|| index.checked_sub(1).and_then(|prev| tabs.get(prev)))
        .map(|tab| tab.session.clone())
}

pub(crate) fn remove_document_tab_with_order(
    tabs: &mut Vec<EditorDocumentTab>,
    visible_order: &mut Vec<EditorTabKey>,
    doc_id: DocId,
) -> Option<DocId> {
    if !tabs.iter().any(|tab| tab.doc_id == doc_id) {
        return None;
    }
    let key = EditorTabKey::Document(doc_id);
    let visible_index = visible_order.iter().position(|existing| *existing == key);
    tabs.retain(|tab| tab.doc_id != doc_id);
    remove_visible_tab_order(visible_order, &key);
    visible_index
        .and_then(|index| next_document_from_visible_order(visible_order, tabs, index))
        .or_else(|| tabs.first().map(|tab| tab.doc_id))
}

pub(crate) fn remove_diff_tab_with_order(
    tabs: &mut Vec<EditorDiffTab>,
    visible_order: &mut Vec<EditorTabKey>,
    key: &str,
) -> Option<DiffSessionWire> {
    if !tabs.iter().any(|tab| tab.key == key) {
        return None;
    }
    let tab_key = EditorTabKey::Diff(key.to_string());
    let visible_index = visible_order
        .iter()
        .position(|existing| *existing == tab_key);
    tabs.retain(|tab| tab.key != key);
    remove_visible_tab_order(visible_order, &tab_key);
    visible_index
        .and_then(|index| next_diff_from_visible_order(visible_order, tabs, index))
        .or_else(|| tabs.first().map(|tab| tab.session.clone()))
}

fn next_document_from_visible_order(
    visible_order: &[EditorTabKey],
    tabs: &[EditorDocumentTab],
    removed_index: usize,
) -> Option<DocId> {
    visible_order
        .iter()
        .skip(removed_index)
        .find_map(|key| document_key_in_tabs(key, tabs))
        .or_else(|| {
            visible_order
                .iter()
                .take(removed_index.min(visible_order.len()))
                .rev()
                .find_map(|key| document_key_in_tabs(key, tabs))
        })
}

fn next_diff_from_visible_order(
    visible_order: &[EditorTabKey],
    tabs: &[EditorDiffTab],
    removed_index: usize,
) -> Option<DiffSessionWire> {
    visible_order
        .iter()
        .skip(removed_index)
        .find_map(|key| diff_key_in_tabs(key, tabs))
        .or_else(|| {
            visible_order
                .iter()
                .take(removed_index.min(visible_order.len()))
                .rev()
                .find_map(|key| diff_key_in_tabs(key, tabs))
        })
}

fn document_key_in_tabs(key: &EditorTabKey, tabs: &[EditorDocumentTab]) -> Option<DocId> {
    let EditorTabKey::Document(doc_id) = key else {
        return None;
    };
    tabs.iter()
        .any(|tab| tab.doc_id == *doc_id)
        .then_some(*doc_id)
}

fn diff_key_in_tabs(key: &EditorTabKey, tabs: &[EditorDiffTab]) -> Option<DiffSessionWire> {
    let EditorTabKey::Diff(key) = key else {
        return None;
    };
    tabs.iter()
        .find(|tab| tab.key == *key)
        .map(|tab| tab.session.clone())
}
