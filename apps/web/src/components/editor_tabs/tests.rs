use super::{
    EditorDocumentTab, EditorTabKey,
    close::close_diff_tab,
    diff_tab_from_session,
    model::display_name,
    ops::{remove_diff_tab, remove_document_tab, upsert_document_tab},
    policy::active_editor_tab_key,
    policy::scope_changed,
    policy::should_clear_diff_on_document_change,
};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use deve_core::models::DocId;
use leptos::prelude::{GetUntracked, signal};

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
fn mobile_surface_close_diff_keeps_source_control_state() {
    let runtime = leptos::reactive::owner::Owner::new();
    runtime.set();
    let first = diff_tab_from_session(DiffSessionWire::new(
        "a.md".into(),
        "old-a".into(),
        "new-a".into(),
    ));
    let second = diff_tab_from_session(DiffSessionWire::new(
        "b.md".into(),
        "old-b".into(),
        "new-b".into(),
    ));
    let active_key = first.key.clone();
    let source_control_state = (1usize, 2usize, "commit message".to_string());
    let (source_control_state_signal, _set_source_control_state) =
        signal(source_control_state.clone());
    let (diff_content, set_diff_content) = signal(Some(first.session.clone()));
    let (diff_tabs, set_diff_tabs) = signal(vec![first, second.clone()]);

    close_diff_tab(
        active_key,
        diff_content,
        set_diff_content,
        diff_tabs,
        set_diff_tabs,
    );

    assert_eq!(
        diff_content
            .get_untracked()
            .as_ref()
            .map(|session| session.path.as_str()),
        Some(second.session.path.as_str())
    );
    assert_eq!(
        source_control_state_signal.get_untracked(),
        source_control_state
    );
}

#[test]
fn editor_tab_runtime_resets_on_repo_or_scope_change() {
    let original = (Some("repo-a".to_string()), 1);

    assert!(!scope_changed(&original, &(Some("repo-a".to_string()), 1)));
    assert!(scope_changed(&original, &(Some("repo-b".to_string()), 1)));
    assert!(scope_changed(&original, &(Some("repo-a".to_string()), 2)));
}

#[test]
fn editor_tab_runtime_clears_diff_only_when_document_changes() {
    let first = Some(DocId::from_u128(1));
    let second = Some(DocId::from_u128(2));

    assert!(should_clear_diff_on_document_change(first, second, true));
    assert!(!should_clear_diff_on_document_change(first, first, true));
    assert!(!should_clear_diff_on_document_change(first, second, false));
}

#[test]
fn editor_tab_runtime_prefers_active_diff_over_document() {
    let doc_id = DocId::from_u128(10);
    let session = DiffSessionWire::new("notes/a.md".into(), "old".into(), "new".into())
        .with_doc_id(Some(doc_id));

    assert_eq!(
        active_editor_tab_key(Some(&session), Some(doc_id)),
        Some(EditorTabKey::Diff(format!("doc:{doc_id}")))
    );
    assert_eq!(
        active_editor_tab_key(None, Some(doc_id)),
        Some(EditorTabKey::Document(doc_id))
    );
}
