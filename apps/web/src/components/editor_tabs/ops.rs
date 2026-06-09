//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use super::model::{DropPosition, EditorDiffTab, EditorDocumentTab, EditorTabItem, EditorTabKey};
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

pub(crate) fn upsert_visible_tab_order(order: &mut Vec<EditorTabKey>, key: EditorTabKey) {
    if !order.contains(&key) {
        order.push(key);
    }
}

pub(crate) fn remove_visible_tab_order(order: &mut Vec<EditorTabKey>, key: &EditorTabKey) {
    order.retain(|existing| existing != key);
}

pub(crate) fn touch_document_access_order(order: &mut Vec<DocId>, doc_id: DocId) {
    order.retain(|existing| *existing != doc_id);
    order.insert(0, doc_id);
}

pub(crate) fn prune_document_access_order(order: &mut Vec<DocId>, tabs: &[EditorDocumentTab]) {
    order.retain(|doc_id| tabs.iter().any(|tab| tab.doc_id == *doc_id));
}

pub(crate) fn ordered_editor_tab_items(
    order: &[EditorTabKey],
    doc_tabs: &[EditorDocumentTab],
    diff_tabs: &[EditorDiffTab],
) -> Vec<EditorTabItem> {
    let mut items = Vec::new();
    for key in order {
        match key {
            EditorTabKey::Document(doc_id) => {
                if let Some(tab) = doc_tabs.iter().find(|tab| tab.doc_id == *doc_id) {
                    items.push(EditorTabItem::Document(tab.clone()));
                }
            }
            EditorTabKey::Diff(key) => {
                if let Some(tab) = diff_tabs.iter().find(|tab| tab.key == *key) {
                    items.push(EditorTabItem::Diff(tab.clone()));
                }
            }
        }
    }
    for tab in doc_tabs {
        let key = EditorTabKey::Document(tab.doc_id);
        if !order.contains(&key) {
            items.push(EditorTabItem::Document(tab.clone()));
        }
    }
    for tab in diff_tabs {
        let key = EditorTabKey::Diff(tab.key.clone());
        if !order.contains(&key) {
            items.push(EditorTabItem::Diff(tab.clone()));
        }
    }
    items
}

pub(crate) fn evict_lru_document_tab(
    tabs: &mut Vec<EditorDocumentTab>,
    visible_order: &mut Vec<EditorTabKey>,
    access_order: &mut Vec<DocId>,
    active_doc: Option<DocId>,
    max_tabs: usize,
) -> Vec<DocId> {
    let max_tabs = max_tabs.max(1);
    prune_document_access_order(access_order, tabs);
    let mut evicted = Vec::new();
    while tabs.len() > max_tabs {
        let Some(candidate) = access_order
            .iter()
            .rev()
            .copied()
            .find(|doc_id| Some(*doc_id) != active_doc)
            .or_else(|| {
                tabs.iter()
                    .find(|tab| Some(tab.doc_id) != active_doc)
                    .map(|tab| tab.doc_id)
            })
        else {
            break;
        };
        tabs.retain(|tab| tab.doc_id != candidate);
        remove_visible_tab_order(visible_order, &EditorTabKey::Document(candidate));
        access_order.retain(|doc_id| *doc_id != candidate);
        evicted.push(candidate);
    }
    evicted
}

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

pub(crate) fn reorder_visible_tab(
    order: &mut Vec<EditorTabKey>,
    dragged: &EditorTabKey,
    target: &EditorTabKey,
    position: DropPosition,
) -> bool {
    if dragged == target {
        return false;
    }
    let Some(from_index) = order.iter().position(|key| key == dragged) else {
        return false;
    };
    if !order.contains(target) {
        return false;
    }
    let dragged_key = order.remove(from_index);
    let Some(target_index) = order.iter().position(|key| key == target) else {
        order.insert(from_index.min(order.len()), dragged_key);
        return false;
    };
    let insert_index = match position {
        DropPosition::Before => target_index,
        DropPosition::After => target_index + 1,
    }
    .min(order.len());
    order.insert(insert_index, dragged_key);
    true
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
