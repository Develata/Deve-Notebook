use crate::components::editor_tabs::{diff_tab_from_session, model::display_name};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::models::DocId;

#[test]
fn display_name_uses_last_path_segment() {
    assert_eq!(display_name("coding/Common Lisp.md"), "Common Lisp.md");
    assert_eq!(display_name("thesis\\main.tex"), "main.tex");
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
