// apps/web/src/components/dashboard/actions_card.rs
//! # Actions Card (快捷操作卡片)
//!
//! 提供 "New Doc" 和 "Sync Now" 按钮。

use crate::hooks::use_core::CoreState;
use crate::hooks::use_core::doc_name::next_untitled_doc_name;
use crate::hooks::use_core::write_gate::repo_write_allowed_for_core_tracked;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn ActionsCard() -> impl IntoView {
    let core = expect_context::<CoreState>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let core_for_gate = core.clone();
    let can_write = Signal::derive(move || repo_write_allowed_for_core_tracked(&core_for_gate));
    let core_for_create = core.clone();

    let on_new_doc = move |_| {
        if !can_write.get_untracked() {
            return;
        }
        let name = next_untitled_doc_name(
            core_for_create
                .docs
                .get_untracked()
                .iter()
                .map(|(_, path)| path.as_str()),
        );
        core_for_create.on_doc_create.run(name);
    };

    let on_sync = move |_| {
        core.on_get_sync_mode.run(());
    };

    view! {
        <div class="bg-panel rounded-lg border border-default p-4">
            <h3 class="text-sm font-semibold text-secondary mb-3">{move || t::dashboard::quick_actions(locale.get())}</h3>
            <div class="flex gap-2">
                <button
                    class="flex-1 px-3 py-2 text-xs font-medium rounded-md \
                           bg-accent text-on-accent hover:bg-accent/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    disabled=move || !can_write.get()
                    on:click=on_new_doc
                >
                    {move || t::dashboard::new_doc(locale.get())}
                </button>
                <button
                    class="flex-1 px-3 py-2 text-xs font-medium rounded-md \
                           border border-default text-primary hover:bg-active transition-colors"
                    on:click=on_sync
                >
                    {move || t::dashboard::sync_now(locale.get())}
                </button>
            </div>
        </div>
    }
}
