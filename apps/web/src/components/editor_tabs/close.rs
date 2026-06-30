//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use super::EditorTabKey;
use super::model::{EditorDiffTab, EditorDocumentTab, diff_tab_key};
use super::ops::{remove_diff_tab_with_order, remove_document_tab_with_order};
use crate::hooks::use_core::EditorContext;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::hooks::use_core::navigation::{NavigationTarget, guard_navigation};
use crate::runtime::{
    document_client::DocumentClient, scope_client::ScopeClient,
    source_control_client::SourceControlClient,
};
use deve_core::models::DocId;
use leptos::prelude::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn build_close_document_callback(
    document: &DocumentClient,
    editor: &EditorContext,
    scope: &ScopeClient,
    source_control: &SourceControlClient,
    doc_tabs: ReadSignal<Vec<EditorDocumentTab>>,
    set_doc_tabs: WriteSignal<Vec<EditorDocumentTab>>,
    tab_order: ReadSignal<Vec<EditorTabKey>>,
    set_tab_order: WriteSignal<Vec<EditorTabKey>>,
    doc_access_order: ReadSignal<Vec<DocId>>,
    set_doc_access_order: WriteSignal<Vec<DocId>>,
) -> Callback<DocId> {
    let current_doc = document.current_doc;
    let current_repo_id = scope.current_repo_id;
    let current_scope_nonce = scope.current_scope_nonce;
    let pending_local_edits = document.pending_local_edits;
    let set_pending_navigation = editor.set_pending_navigation;
    let set_current_doc = document.set_current_doc;
    let set_explicit_home = document.set_explicit_home;
    let diff_content = source_control.diff_content;
    let set_diff_content = source_control.set_diff_content;

    Callback::new(move |doc_id| {
        if current_doc.get_untracked() != Some(doc_id) {
            let mut next_tabs = doc_tabs.get_untracked();
            let mut next_order = tab_order.get_untracked();
            let _ = remove_document_tab_with_order(&mut next_tabs, &mut next_order, doc_id);
            set_doc_tabs.set(next_tabs);
            set_tab_order.set(next_order);
            set_doc_access_order.update(|order| order.retain(|existing| *existing != doc_id));
            return;
        }

        let mut next_tabs = doc_tabs.get_untracked();
        let mut next_order = tab_order.get_untracked();
        let mut next_access_order = doc_access_order.get_untracked();
        let next_doc = remove_document_tab_with_order(&mut next_tabs, &mut next_order, doc_id);
        next_access_order.retain(|existing| *existing != doc_id);
        let active_diff = diff_content.get_untracked();
        let action = Callback::new(move |_| {
            set_doc_tabs.set(next_tabs.clone());
            set_tab_order.set(next_order.clone());
            set_doc_access_order.set(next_access_order.clone());
            apply_document_close_result(
                active_diff.clone(),
                next_doc,
                set_current_doc,
                set_explicit_home,
                set_diff_content,
            );
        });

        let target = if next_doc.is_some() {
            NavigationTarget::Doc
        } else {
            NavigationTarget::Home
        };
        let _ = guard_navigation(
            current_doc.get_untracked(),
            current_repo_id.get_untracked().as_deref(),
            current_scope_nonce.get_untracked(),
            &pending_local_edits.get_untracked(),
            set_pending_navigation,
            target,
            action,
        );
    })
}

fn apply_document_close_result(
    active_diff: Option<DiffSessionWire>,
    next_doc: Option<DocId>,
    set_current_doc: WriteSignal<Option<DocId>>,
    set_explicit_home: WriteSignal<bool>,
    set_diff_content: WriteSignal<Option<DiffSessionWire>>,
) {
    match (active_diff, next_doc) {
        (Some(session), next_doc) => {
            set_current_doc.set(next_doc);
            set_explicit_home.set(false);
            set_diff_content.set(Some(session));
        }
        (None, Some(next_doc)) => {
            set_explicit_home.set(false);
            set_diff_content.set(None);
            set_current_doc.set(Some(next_doc));
        }
        (None, None) => {
            set_diff_content.set(None);
            set_current_doc.set(None);
            set_explicit_home.set(true);
        }
    }
}

pub(super) fn close_diff_tab(
    key: String,
    diff_content: ReadSignal<Option<DiffSessionWire>>,
    set_diff_content: WriteSignal<Option<DiffSessionWire>>,
    diff_tabs: ReadSignal<Vec<EditorDiffTab>>,
    set_diff_tabs: WriteSignal<Vec<EditorDiffTab>>,
    tab_order: ReadSignal<Vec<EditorTabKey>>,
    set_tab_order: WriteSignal<Vec<EditorTabKey>>,
) {
    let is_active = diff_content
        .get_untracked()
        .is_some_and(|session| diff_tab_key(&session) == key);
    let mut next_tabs = diff_tabs.get_untracked();
    let mut next_order = tab_order.get_untracked();
    let next_session = remove_diff_tab_with_order(&mut next_tabs, &mut next_order, &key);
    set_diff_tabs.set(next_tabs);
    set_tab_order.set(next_order);
    if is_active {
        set_diff_content.set(next_session);
    }
}

#[cfg(test)]
#[path = "close_tests.rs"]
mod close_tests;
