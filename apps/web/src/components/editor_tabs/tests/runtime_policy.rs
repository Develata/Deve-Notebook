use crate::components::editor_tabs::{
    EditorTabKey, diff_tab_from_session,
    policy::{
        active_editor_tab_key, editor_tab_runtime_scope, scope_changed,
        should_clear_diff_on_document_change,
    },
};
use crate::runtime::source_control_client::diff_session::DiffSessionWire;
use deve_core::models::{DocId, PeerId};

#[test]
fn editor_tab_runtime_resets_on_repo_branch_or_scope_change() {
    let original = editor_tab_runtime_scope(Some("repo-a".to_string()), 1, None);
    let shadow_branch =
        editor_tab_runtime_scope(Some("repo-a".to_string()), 1, Some(PeerId::new("peer-a")));

    assert!(!scope_changed(
        &original,
        &editor_tab_runtime_scope(Some("repo-a".to_string()), 1, None),
    ));
    assert!(scope_changed(
        &original,
        &editor_tab_runtime_scope(Some("repo-b".to_string()), 1, None),
    ));
    assert!(scope_changed(
        &original,
        &editor_tab_runtime_scope(Some("repo-a".to_string()), 2, None),
    ));
    assert!(scope_changed(&original, &shadow_branch));
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

#[test]
fn active_diff_key_matches_projected_diff_tab_key() {
    let doc_id = DocId::from_u128(11);
    let session = DiffSessionWire::new("notes/b.md".into(), "old".into(), "new".into())
        .with_doc_id(Some(doc_id));
    let tab = diff_tab_from_session(session.clone());

    assert_eq!(
        active_editor_tab_key(Some(&session), Some(doc_id)),
        Some(EditorTabKey::Diff(tab.key))
    );
}
