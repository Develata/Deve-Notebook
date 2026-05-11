//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime
//!
use crate::hooks::use_core::navigation::{NavigationTarget, PendingNavigation, guard_navigation};
use crate::hooks::use_core::pending::PendingLocalEdits;
use deve_core::models::DocId;
use leptos::prelude::*;

pub(super) fn create_doc_select_callback(
    current_doc: ReadSignal<Option<DocId>>,
    current_repo_id: ReadSignal<Option<String>>,
    current_scope_nonce: ReadSignal<u64>,
    pending_local_edits: ReadSignal<PendingLocalEdits>,
    set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    set_current_doc: WriteSignal<Option<DocId>>,
    set_explicit_home: WriteSignal<bool>,
) -> Callback<DocId> {
    Callback::new(move |id: DocId| {
        if current_doc.get_untracked() == Some(id) {
            set_explicit_home.set(false);
            return;
        }
        let action = Callback::new(move |_: ()| {
            set_explicit_home.set(false);
            set_current_doc.set(Some(id));
        });
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
