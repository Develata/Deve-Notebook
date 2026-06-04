//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip

use super::editor_tabs::{
    EditorDiffTab, EditorDocumentTab, EditorTabKey, diff_tab_from_session, diff_tab_key,
    document_tab_from_docs, remove_diff_tab, remove_document_tab, upsert_diff_tab,
    upsert_document_tab,
};
use crate::hooks::use_core::CoreState;
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::hooks::use_core::navigation::{NavigationTarget, guard_navigation};
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

pub(crate) fn create_current_editor_doc(core: &CoreState) -> Signal<Option<DocId>> {
    let current_doc = core.current_doc;
    let pending_branch_switch = core.pending_branch_switch;
    let pending_repo_switch = core.pending_repo_switch;
    Signal::derive(move || {
        if pending_branch_switch.get().is_some() || pending_repo_switch.get().is_some() {
            None
        } else {
            current_doc.get()
        }
    })
}

pub(crate) fn create_editor_tab_runtime(
    core: &CoreState,
    current_editor_doc: Signal<Option<DocId>>,
) -> EditorTabRuntime {
    let (doc_tabs, set_doc_tabs) = signal(Vec::<EditorDocumentTab>::new());
    let (diff_tabs, set_diff_tabs) = signal(Vec::<EditorDiffTab>::new());
    let diff_content = core.diff_content;
    let set_diff_content = core.set_diff_content;
    let current_repo_id = core.current_repo_id;
    let current_scope_nonce = core.current_scope_nonce;
    let current_doc = core.current_doc;
    let last_scope = StoredValue::new((
        current_repo_id.get_untracked(),
        current_scope_nonce.get_untracked(),
    ));
    let last_current_doc = StoredValue::new(current_doc.get_untracked());

    Effect::new(move |_| {
        let scope = (current_repo_id.get(), current_scope_nonce.get());
        if last_scope.get_value() == scope {
            return;
        }
        last_scope.set_value(scope);
        set_doc_tabs.set(Vec::new());
        set_diff_tabs.set(Vec::new());
        set_diff_content.set(None);
    });

    Effect::new(move |_| {
        let next = current_doc.get();
        if last_current_doc.get_value() == next {
            return;
        }
        last_current_doc.set_value(next);
        if diff_content.get_untracked().is_some() {
            set_diff_content.set(None);
        }
    });

    let docs = core.docs;
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
        diff_content
            .get()
            .map(|session| EditorTabKey::Diff(diff_tab_key(&session)))
            .or_else(|| current_editor_doc.get().map(EditorTabKey::Document))
    });

    EditorTabRuntime {
        doc_tabs,
        diff_tabs,
        active_tab,
        on_select_document: build_select_document_callback(core),
        on_select_diff: Callback::new(move |session| set_diff_content.set(Some(session))),
        on_close_document: build_close_document_callback(core, doc_tabs, set_doc_tabs, diff_tabs),
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

fn build_select_document_callback(core: &CoreState) -> Callback<DocId> {
    let current_doc = core.current_doc;
    let current_repo_id = core.current_repo_id;
    let current_scope_nonce = core.current_scope_nonce;
    let pending_local_edits = core.pending_local_edits;
    let set_pending_navigation = core.set_pending_navigation;
    let set_current_doc = core.set_current_doc;
    let set_explicit_home = core.set_explicit_home;
    let set_diff_content = core.set_diff_content;

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

fn build_close_document_callback(
    core: &CoreState,
    doc_tabs: ReadSignal<Vec<EditorDocumentTab>>,
    set_doc_tabs: WriteSignal<Vec<EditorDocumentTab>>,
    diff_tabs: ReadSignal<Vec<EditorDiffTab>>,
) -> Callback<DocId> {
    let current_doc = core.current_doc;
    let current_repo_id = core.current_repo_id;
    let current_scope_nonce = core.current_scope_nonce;
    let pending_local_edits = core.pending_local_edits;
    let set_pending_navigation = core.set_pending_navigation;
    let set_current_doc = core.set_current_doc;
    let set_explicit_home = core.set_explicit_home;
    let diff_content = core.diff_content;
    let set_diff_content = core.set_diff_content;

    Callback::new(move |doc_id| {
        if current_doc.get_untracked() != Some(doc_id) {
            set_doc_tabs.update(|tabs| {
                let _ = remove_document_tab(tabs, doc_id);
            });
            return;
        }

        let mut next_tabs = doc_tabs.get_untracked();
        let next_doc = remove_document_tab(&mut next_tabs, doc_id);
        let active_diff = diff_content.get_untracked();
        let fallback_diff = if active_diff.is_none() {
            diff_tabs
                .get_untracked()
                .first()
                .map(|tab| tab.session.clone())
        } else {
            None
        };
        let action = Callback::new(move |_| {
            set_doc_tabs.set(next_tabs.clone());
            apply_document_close_result(
                active_diff.clone(),
                next_doc,
                fallback_diff.clone(),
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
    fallback_diff: Option<DiffSessionWire>,
    set_current_doc: WriteSignal<Option<DocId>>,
    set_explicit_home: WriteSignal<bool>,
    set_diff_content: WriteSignal<Option<DiffSessionWire>>,
) {
    match (active_diff, next_doc, fallback_diff) {
        (Some(session), next_doc, _) => {
            set_current_doc.set(next_doc);
            set_explicit_home.set(false);
            set_diff_content.set(Some(session));
        }
        (None, Some(next_doc), _) => {
            set_explicit_home.set(false);
            set_diff_content.set(None);
            set_current_doc.set(Some(next_doc));
        }
        (None, None, Some(session)) => {
            set_explicit_home.set(false);
            set_current_doc.set(None);
            set_diff_content.set(Some(session));
        }
        (None, None, None) => {
            set_diff_content.set(None);
            set_current_doc.set(None);
            set_explicit_home.set(true);
        }
    }
}

fn close_diff_tab(
    key: String,
    diff_content: ReadSignal<Option<DiffSessionWire>>,
    set_diff_content: WriteSignal<Option<DiffSessionWire>>,
    diff_tabs: ReadSignal<Vec<EditorDiffTab>>,
    set_diff_tabs: WriteSignal<Vec<EditorDiffTab>>,
) {
    let is_active = diff_content
        .get_untracked()
        .is_some_and(|session| diff_tab_key(&session) == key);
    let mut next_tabs = diff_tabs.get_untracked();
    let next_session = remove_diff_tab(&mut next_tabs, &key);
    set_diff_tabs.set(next_tabs);
    if is_active {
        set_diff_content.set(next_session);
    }
}
