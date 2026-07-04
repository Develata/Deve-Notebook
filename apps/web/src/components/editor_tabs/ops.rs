//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

mod removal;

#[cfg(test)]
pub(crate) use removal::{remove_diff_tab, remove_document_tab};
pub(crate) use removal::{remove_diff_tab_with_order, remove_document_tab_with_order};

use super::model::{
    DropPosition, EditorDiffTab, EditorDocumentTab, EditorTabItem, EditorTabKey, display_name,
};
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

pub(crate) fn reconcile_document_tabs_with_docs(
    tabs: &mut Vec<EditorDocumentTab>,
    visible_order: &mut Vec<EditorTabKey>,
    access_order: &mut Vec<DocId>,
    docs: &[(DocId, String)],
) -> bool {
    let mut changed = false;
    for tab in tabs.iter_mut() {
        let Some((_, path)) = docs.iter().find(|(doc_id, _)| *doc_id == tab.doc_id) else {
            continue;
        };
        let title = display_name(path);
        if tab.title != title || tab.tooltip != *path {
            tab.title = title;
            tab.tooltip.clone_from(path);
            changed = true;
        }
    }

    let before_len = tabs.len();
    let mut removed = Vec::new();
    tabs.retain(|tab| {
        let keep = docs.iter().any(|(doc_id, _)| *doc_id == tab.doc_id);
        if !keep {
            removed.push(tab.doc_id);
        }
        keep
    });
    if tabs.len() != before_len {
        changed = true;
    }
    for doc_id in removed {
        remove_visible_tab_order(visible_order, &EditorTabKey::Document(doc_id));
        access_order.retain(|existing| *existing != doc_id);
    }
    changed
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
