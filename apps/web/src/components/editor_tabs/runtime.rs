//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use super::close::{build_close_document_callback, close_diff_tab};
use super::ops::{upsert_diff_tab, upsert_document_tab};
use super::policy::{active_editor_tab_key, scope_changed, should_clear_diff_on_document_change};
use super::{
    EditorDiffTab, EditorDocumentTab, EditorTabKey, diff_tab_from_session, document_tab_from_docs,
};
use crate::hooks::use_core::EditorContext;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::hooks::use_core::navigation::{NavigationTarget, guard_navigation};
use crate::runtime::{
    document_client::DocumentClient, scope_client::ScopeClient,
    source_control_client::SourceControlClient,
};
use deve_core::models::DocId;
use leptos::prelude::*;

pub(crate) struct EditorTabRuntime {
    pub doc_tabs: ReadSignal<Vec<EditorDocumentTab>>,
    pub diff_tabs: ReadSignal<Vec<EditorDiffTab>>,
    pub active_tab: Signal<Option<EditorTabKey>>,
    pub on_select_document: Callback<DocId>,
    pub on_select_diff: Callback<DiffSessionWire>,
    pub on_close_document: Callback<DocId>,
    pub on_close_diff: Callback<String>,
}

#[derive(Clone)]
pub(crate) struct EditorTabRuntimeInputs {
    pub document: DocumentClient,
    pub editor: EditorContext,
    pub scope: ScopeClient,
    pub source_control: SourceControlClient,
}

pub(crate) fn create_current_editor_doc(
    document: &DocumentClient,
    editor: &EditorContext,
) -> Signal<Option<DocId>> {
    let current_doc = document.current_doc;
    let pending_branch_switch = editor.pending_branch_switch;
    let pending_repo_switch = editor.pending_repo_switch;
    Signal::derive(move || {
        if pending_branch_switch.get().is_some() || pending_repo_switch.get().is_some() {
            None
        } else {
            current_doc.get()
        }
    })
}

pub(crate) fn create_editor_tab_runtime(
    inputs: EditorTabRuntimeInputs,
    current_editor_doc: Signal<Option<DocId>>,
) -> EditorTabRuntime {
    let (doc_tabs, set_doc_tabs) = signal(Vec::<EditorDocumentTab>::new());
    let (diff_tabs, set_diff_tabs) = signal(Vec::<EditorDiffTab>::new());
    let diff_content = inputs.source_control.diff_content;
    let set_diff_content = inputs.source_control.set_diff_content;
    let current_repo_id = inputs.scope.current_repo_id;
    let current_scope_nonce = inputs.scope.current_scope_nonce;
    let current_doc = inputs.document.current_doc;
    let last_scope = StoredValue::new((
        current_repo_id.get_untracked(),
        current_scope_nonce.get_untracked(),
    ));
    let last_current_doc = StoredValue::new(current_doc.get_untracked());

    Effect::new(move |_| {
        let scope = (current_repo_id.get(), current_scope_nonce.get());
        if !scope_changed(&last_scope.get_value(), &scope) {
            return;
        }
        last_scope.set_value(scope);
        set_doc_tabs.set(Vec::new());
        set_diff_tabs.set(Vec::new());
        set_diff_content.set(None);
    });

    Effect::new(move |_| {
        let next = current_doc.get();
        let previous = last_current_doc.get_value();
        if previous == next {
            return;
        }
        last_current_doc.set_value(next);
        if should_clear_diff_on_document_change(
            previous,
            next,
            diff_content.get_untracked().is_some(),
        ) {
            set_diff_content.set(None);
        }
    });

    let docs = inputs.document.docs;
    Effect::new(move |_| {
        if let Some(doc_id) = current_editor_doc.get()
            && let Some(tab) = document_tab_from_docs(&docs.get(), doc_id)
        {
            set_doc_tabs.update(|tabs| upsert_document_tab(tabs, tab));
        }
    });

    Effect::new(move |_| {
        if let Some(session) = diff_content.get() {
            set_diff_tabs.update(|tabs| upsert_diff_tab(tabs, diff_tab_from_session(session)));
        }
    });

    let active_tab = Signal::derive(move || {
        active_editor_tab_key(diff_content.get().as_ref(), current_editor_doc.get())
    });

    EditorTabRuntime {
        doc_tabs,
        diff_tabs,
        active_tab,
        on_select_document: build_select_document_callback(
            &inputs.document,
            &inputs.editor,
            &inputs.scope,
            &inputs.source_control,
        ),
        on_select_diff: Callback::new(move |session| set_diff_content.set(Some(session))),
        on_close_document: build_close_document_callback(
            &inputs.document,
            &inputs.editor,
            &inputs.scope,
            &inputs.source_control,
            doc_tabs,
            set_doc_tabs,
            diff_tabs,
        ),
        on_close_diff: Callback::new(move |key| {
            close_diff_tab(
                key,
                diff_content,
                set_diff_content,
                diff_tabs,
                set_diff_tabs,
            );
        }),
    }
}

fn build_select_document_callback(
    document: &DocumentClient,
    editor: &EditorContext,
    scope: &ScopeClient,
    source_control: &SourceControlClient,
) -> Callback<DocId> {
    let current_doc = document.current_doc;
    let current_repo_id = scope.current_repo_id;
    let current_scope_nonce = scope.current_scope_nonce;
    let pending_local_edits = document.pending_local_edits;
    let set_pending_navigation = editor.set_pending_navigation;
    let set_current_doc = document.set_current_doc;
    let set_explicit_home = document.set_explicit_home;
    let set_diff_content = source_control.set_diff_content;

    Callback::new(move |doc_id| {
        let action = Callback::new(move |_| {
            set_explicit_home.set(false);
            set_diff_content.set(None);
            set_current_doc.set(Some(doc_id));
        });
        if current_doc.get_untracked() == Some(doc_id) {
            action.run(());
            return;
        }
        let _ = guard_navigation(
            current_doc.get_untracked(),
            current_repo_id.get_untracked().as_deref(),
            current_scope_nonce.get_untracked(),
            &pending_local_edits.get_untracked(),
            set_pending_navigation,
            NavigationTarget::Doc,
            action,
        );
    })
}
