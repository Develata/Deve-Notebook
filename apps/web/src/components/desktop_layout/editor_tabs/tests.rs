use super::{
    EditorDocumentTab, diff_tab_from_session, model::display_name, remove_diff_tab,
    remove_document_tab, strip::tab_button_class, upsert_document_tab,
};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::models::DocId;

#[test]
fn display_name_uses_last_path_segment() {
    assert_eq!(display_name("coding/Common Lisp.md"), "Common Lisp.md");
    assert_eq!(display_name("thesis\\main.tex"), "main.tex");
}

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
        EditorDocumentTab {
            doc_id: first,
            title: "a.md".into(),
            tooltip: "a.md".into(),
        },
        EditorDocumentTab {
            doc_id: second,
            title: "b.md".into(),
            tooltip: "b.md".into(),
        },
        EditorDocumentTab {
            doc_id: third,
            title: "c.md".into(),
            tooltip: "c.md".into(),
        },
    ];

    assert_eq!(remove_document_tab(&mut tabs, second), Some(third));
    assert_eq!(remove_document_tab(&mut tabs, third), Some(first));
    assert_eq!(remove_document_tab(&mut tabs, first), None);
}

#[test]
fn diff_tabs_use_display_path_for_title_and_stable_doc_key() {
    let doc_id = DocId::from_u128(7);
    let tab = diff_tab_from_session(
        DiffSessionWire::with_display_path(
            "notes/new.md".into(),
            "notes/old.md -> notes/new.md".into(),
            "old".into(),
            "new".into(),
        )
        .with_doc_id(Some(doc_id)),
    );

    assert_eq!(tab.key, format!("doc:{doc_id}"));
    assert_eq!(tab.title, "new.md");
    assert!(tab.tooltip.contains("notes/new.md"));
}

#[test]
fn removing_diff_tab_returns_neighbor_session() {
    let mut tabs = vec![
        diff_tab_from_session(DiffSessionWire::new(
            "a.md".into(),
            "old".into(),
            "new".into(),
        )),
        diff_tab_from_session(DiffSessionWire::new(
            "b.md".into(),
            "old".into(),
            "new".into(),
        )),
    ];
    let first_key = tabs[0].key.clone();
    let second_path = tabs[1].session.path.clone();

    assert_eq!(
        remove_diff_tab(&mut tabs, &first_key)
            .expect("neighbor")
            .path,
        second_path
    );
}

#[test]
fn active_tab_class_has_accent_top_border() {
    assert!(tab_button_class(true).contains("border-t-accent"));
    assert!(tab_button_class(false).contains("border-t-transparent"));
}
