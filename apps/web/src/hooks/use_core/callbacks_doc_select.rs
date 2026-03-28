use crate::hooks::use_core::navigation::{NavigationTarget, PendingNavigation, guard_navigation};
use crate::hooks::use_core::pending::PendingLocalEdits;
use deve_core::models::DocId;
use leptos::prelude::*;

pub(super) fn create_doc_select_callback(
    current_doc: ReadSignal<Option<DocId>>,
    pending_local_edits: ReadSignal<PendingLocalEdits>,
    set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    set_current_doc: WriteSignal<Option<DocId>>,
    set_explicit_home: WriteSignal<bool>,
) -> Callback<DocId> {
    Callback::new(move |id: DocId| {
        if current_doc.get_untracked() == Some(id) {
            set_explicit_home.set(false);
            set_current_doc.set(Some(id));
            return;
        }
        let action = Callback::new(move |_: ()| {
            set_explicit_home.set(false);
            set_current_doc.set(Some(id));
        });
        let _ = guard_navigation(
            current_doc.get_untracked(),
            &pending_local_edits.get_untracked(),
            set_pending_navigation,
            NavigationTarget::Doc,
            action,
        );
    })
}
