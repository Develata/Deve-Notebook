// apps/web/src/components/mobile_layout/content.rs
//! # Mobile Content

use crate::components::dashboard::Dashboard;
use crate::editor::Editor;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn MobileContent(core: CoreState, drawer_open: Signal<bool>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let editor_doc_core = core.clone();
    let current_editor_doc = Signal::derive(move || {
        if editor_doc_core.pending_branch_switch.get().is_some()
            || editor_doc_core.pending_repo_switch.get().is_some()
        {
            None
        } else {
            editor_doc_core.current_doc.get()
        }
    });
    let diff_core = core.clone();
    let editor_core = core.clone();
    view! {
        <div
            class="relative flex-1 overflow-hidden transition-opacity flex flex-col"
            class:pointer-events-none=move || drawer_open.get()
            class:opacity-80=move || drawer_open.get()
        >
            <Show when=move || core.is_spectator.get() && core.sync_banner.get().is_none()>
                <div class="h-6 px-3 flex items-center text-[11px] font-medium text-orange-900 bg-orange-200 border-b border-orange-300">
                    {move || t::common::read_only_mode(locale.get())}
                </div>
            </Show>
            <div class="flex-1 min-h-0 overflow-hidden">
                <Show
                    when=move || diff_core.diff_content.get().is_some()
                    fallback=move || view! {
                        <Show
                            when=move || current_editor_doc.get().is_some()
                            fallback=move || view! { <Dashboard /> }
                        >
                            {move || {
                                current_editor_doc
                                    .get()
                                    .map(|doc_id| view! { <Editor doc_id=doc_id on_stats=editor_core.on_stats embedded=true /> })
                            }}
                        </Show>
                    }
                >
                    {move || {
                        diff_core.diff_content.get().map(|session| {
                            view! {
                                <crate::components::diff_view::DiffView
                                    repo_scope=diff_core
                                        .current_repo_id
                                        .get()
                                        .or_else(|| diff_core.current_repo.get())
                                        .unwrap_or_default()
                                    path=session.path
                                    display_path=session.display_path
                                    old_content=session.old_content
                                    new_content=session.new_content
                                    is_readonly=diff_core.is_spectator.get()
                                    force_unified=true
                                    mobile=true
                                    on_close=Callback::new(move |_| diff_core.set_diff_content.set(None))
                                />
                            }
                        })
                    }}
                </Show>
            </div>
        </div>
    }
}
