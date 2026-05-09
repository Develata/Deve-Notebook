//! plan_ref:
//!   - 03_rendering#document-authority-bridge
//!   - 06_repository#repo-scope-runtime
//!
use crate::hooks::use_core::navigation::{NavigationTarget, guard_navigation};
use crate::hooks::use_core::pending::PendingLocalEdits;
use leptos::prelude::*;

pub fn toggle_search_callback(
    show_search: ReadSignal<bool>,
    set_show_search: WriteSignal<bool>,
    search_mode: ReadSignal<String>,
    set_search_mode: WriteSignal<String>,
    target_mode: String,
) -> Callback<()> {
    Callback::new(move |_| {
        let is_visible = show_search.get_untracked();
        let mode = search_mode.get_untracked();
        if is_visible && mode == target_mode {
            set_show_search.set(false);
        } else {
            set_search_mode.set(target_mode.clone());
            set_show_search.set(true);
        }
    })
}

pub fn build_open_callback(
    show_search: ReadSignal<bool>,
    set_show_search: WriteSignal<bool>,
    search_mode: ReadSignal<String>,
    set_search_mode: WriteSignal<String>,
) -> Callback<()> {
    toggle_search_callback(
        show_search,
        set_show_search,
        search_mode,
        set_search_mode,
        String::new(),
    )
}

pub fn build_home_callback(
    set_doc: WriteSignal<Option<deve_core::models::DocId>>,
    set_explicit_home: WriteSignal<bool>,
    current_doc: ReadSignal<Option<deve_core::models::DocId>>,
    current_repo_id: ReadSignal<Option<String>>,
    current_scope_nonce: ReadSignal<u64>,
    pending_local_edits: ReadSignal<PendingLocalEdits>,
    set_pending_navigation: WriteSignal<
        Option<crate::hooks::use_core::navigation::PendingNavigation>,
    >,
) -> Callback<()> {
    Callback::new(move |_| {
        let action = Callback::new(move |_: ()| {
            set_explicit_home.set(true);
            set_doc.set(None);
        });
        let _ = guard_navigation(
            current_doc.get_untracked(),
            current_repo_id.get_untracked().as_deref(),
            current_scope_nonce.get_untracked(),
            &pending_local_edits.get_untracked(),
            set_pending_navigation,
            NavigationTarget::Home,
            action,
        );
    })
}
