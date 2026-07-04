use crate::components::editor_tabs::{
    DropPosition, EditorDocumentTab, EditorTabItem, EditorTabKey, diff_tab_from_session,
    ops::{
        evict_lru_document_tab, ordered_editor_tab_items, reconcile_document_tabs_with_docs,
        remove_document_tab, remove_document_tab_with_order, reorder_visible_tab,
        touch_document_access_order, upsert_document_tab, upsert_visible_tab_order,
    },
};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::models::DocId;

#[test]
fn document_tabs_upsert_by_doc_identity() {
    let doc_id = DocId::from_u128(1);
    let mut tabs = Vec::new();
    upsert_document_tab(
        &mut tabs,
        EditorDocumentTab {
            doc_id,
            title: "old.md".into(),
            tooltip: "old.md".into(),
        },
    );
    upsert_document_tab(
        &mut tabs,
        EditorDocumentTab {
            doc_id,
            title: "new.md".into(),
            tooltip: "folder/new.md".into(),
        },
    );

    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].title, "new.md");
    assert_eq!(tabs[0].tooltip, "folder/new.md");
}

#[test]
fn removing_tab_returns_right_neighbor_then_left_neighbor() {
    let first = DocId::from_u128(1);
    let second = DocId::from_u128(2);
    let third = DocId::from_u128(3);
    let mut tabs = vec![
        doc_tab(first, "a.md"),
        doc_tab(second, "b.md"),
        doc_tab(third, "c.md"),
    ];

    assert_eq!(remove_document_tab(&mut tabs, second), Some(third));
    assert_eq!(remove_document_tab(&mut tabs, third), Some(first));
    assert_eq!(remove_document_tab(&mut tabs, first), None);
}

#[test]
fn removing_document_tab_with_visible_order_returns_right_neighbor_then_left_neighbor() {
    let first = DocId::from_u128(1);
    let second = DocId::from_u128(2);
    let third = DocId::from_u128(3);
    let mut tabs = vec![
        doc_tab(first, "a.md"),
        doc_tab(second, "b.md"),
        doc_tab(third, "c.md"),
    ];
    let mut visible_order = vec![
        EditorTabKey::Document(third),
        EditorTabKey::Document(first),
        EditorTabKey::Document(second),
    ];

    assert_eq!(
        remove_document_tab_with_order(&mut tabs, &mut visible_order, first),
        Some(second)
    );
    assert_eq!(
        remove_document_tab_with_order(&mut tabs, &mut visible_order, second),
        Some(third)
    );
    assert_eq!(
        remove_document_tab_with_order(&mut tabs, &mut visible_order, third),
        None
    );
}

#[test]
fn editor_tab_visible_order_tracks_docs_and_diffs_without_touching_lru() {
    let doc_id = DocId::from_u128(1);
    let diff = diff_tab_from_session(DiffSessionWire::new(
        "a.md".into(),
        "old".into(),
        "new".into(),
    ));
    let diff_key = diff.key.clone();
    let doc_tabs = vec![doc_tab(doc_id, "a.md")];
    let diff_tabs = vec![diff];
    let mut visible_order = Vec::new();
    let mut access_order = Vec::new();

    upsert_visible_tab_order(&mut visible_order, EditorTabKey::Document(doc_id));
    upsert_visible_tab_order(&mut visible_order, EditorTabKey::Diff(diff_key.clone()));
    touch_document_access_order(&mut access_order, doc_id);
    assert!(reorder_visible_tab(
        &mut visible_order,
        &EditorTabKey::Diff(diff_key),
        &EditorTabKey::Document(doc_id),
        DropPosition::Before,
    ));

    let items = ordered_editor_tab_items(&visible_order, &doc_tabs, &diff_tabs);
    assert!(matches!(items[0], EditorTabItem::Diff(_)));
    assert_eq!(access_order, vec![doc_id]);
}

#[test]
fn editor_tab_document_lru_evicts_oldest_non_active_document_only() {
    let first = DocId::from_u128(1);
    let second = DocId::from_u128(2);
    let third = DocId::from_u128(3);
    let diff = diff_tab_from_session(DiffSessionWire::new(
        "diff.md".into(),
        "old".into(),
        "new".into(),
    ));
    let diff_key = diff.key.clone();
    let mut tabs = vec![
        doc_tab(first, "a.md"),
        doc_tab(second, "b.md"),
        doc_tab(third, "c.md"),
    ];
    let mut visible_order = vec![
        EditorTabKey::Document(first),
        EditorTabKey::Document(second),
        EditorTabKey::Diff(diff_key.clone()),
        EditorTabKey::Document(third),
    ];
    let mut access_order = vec![third, second, first];

    let evicted = evict_lru_document_tab(
        &mut tabs,
        &mut visible_order,
        &mut access_order,
        Some(first),
        2,
    );

    assert_eq!(evicted, vec![second]);
    assert!(tabs.iter().any(|tab| tab.doc_id == first));
    assert!(tabs.iter().any(|tab| tab.doc_id == third));
    assert!(visible_order.contains(&EditorTabKey::Diff(diff_key)));
    assert!(!access_order.contains(&second));
}

#[test]
fn editor_tab_doc_projection_reconciles_titles_and_removes_deleted_docs() {
    let first = DocId::from_u128(1);
    let second = DocId::from_u128(2);
    let stale = DocId::from_u128(3);
    let mut tabs = vec![
        doc_tab(first, "old-a.md"),
        doc_tab(second, "b.md"),
        doc_tab(stale, "deleted.md"),
    ];
    let mut visible_order = vec![
        EditorTabKey::Document(stale),
        EditorTabKey::Document(first),
        EditorTabKey::Document(second),
    ];
    let mut access_order = vec![stale, second, first];

    let changed = reconcile_document_tabs_with_docs(
        &mut tabs,
        &mut visible_order,
        &mut access_order,
        &[(first, "notes/a.md".into()), (second, "b.md".into())],
    );

    assert!(changed);
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs[0].title, "a.md");
    assert_eq!(tabs[0].tooltip, "notes/a.md");
    assert!(!tabs.iter().any(|tab| tab.doc_id == stale));
    assert!(!visible_order.contains(&EditorTabKey::Document(stale)));
    assert!(!access_order.contains(&stale));
}

#[test]
fn editor_tab_reorder_visible_tab_supports_after_last_position() {
    let first = EditorTabKey::Document(DocId::from_u128(1));
    let second = EditorTabKey::Document(DocId::from_u128(2));
    let third = EditorTabKey::Document(DocId::from_u128(3));
    let mut order = vec![first.clone(), second.clone(), third.clone()];

    assert!(reorder_visible_tab(
        &mut order,
        &first,
        &third,
        DropPosition::After,
    ));

    assert_eq!(order, vec![second, third, first]);
}

fn doc_tab(doc_id: DocId, path: &str) -> EditorDocumentTab {
    EditorDocumentTab {
        doc_id,
        title: path.to_string(),
        tooltip: path.to_string(),
    }
}
