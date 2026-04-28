// apps/web/src/components/mobile_layout/content.rs
//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-responsive-layout
//!   - 03_rendering#large-document-runtime
//!
//! # Mobile Content

use crate::components::dashboard::Dashboard;
use crate::editor::Editor;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn MobileContent(core: CoreState, drawer_open: Signal<bool>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let on_stats = core.on_stats;
    let diff_content = core.diff_content;
    let set_diff_content = core.set_diff_content;
    let current_repo_id = core.current_repo_id;
    let current_repo = core.current_repo;
    let current_scope_nonce = core.current_scope_nonce;
    let is_spectator = core.is_spectator;
    let sync_banner = core.sync_banner;
    let ws = core.ws.clone();
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
    view! {
        <div
            class="relative flex-1 overflow-hidden transition-opacity flex flex-col"
            class:pointer-events-none=move || drawer_open.get()
            class:opacity-80=move || drawer_open.get()
        >
            <Show when=move || is_spectator.get() && sync_banner.get().is_none()>
                <div class="h-6 px-3 flex items-center text-[11px] font-medium text-orange-900 bg-orange-200 border-b border-orange-300">
                    {move || t::common::read_only_mode(locale.get())}
                </div>
            </Show>
            <div class="flex-1 min-h-0 overflow-hidden">
                {move || {
                    if let Some(session) = diff_content.get() {
                            let merge_conflict = session.merge_conflict.clone();
                            let repo_scope = current_repo_id
                                .get()
                                .or_else(|| current_repo.get())
                                .unwrap_or_default();
                            let resolve_ws = ws.clone();
                            let on_resolve = merge_conflict.clone().map(|conflict| {
                                let resolve_ws = resolve_ws.clone();
                                Callback::new(move |(action, result_content)| {
                                    resolve_ws.send(conflict.resolve_message(
                                        action,
                                        result_content,
                                        current_scope_nonce.get_untracked(),
                                    ));
                                    set_diff_content.set(None);
                                })
                            });
                            view! {
                                <crate::components::diff_view::DiffView
                                    repo_scope=repo_scope
                                    path=session.path
                                    display_path=session.display_path
                                    old_content=session.old_content
                                    new_content=session.new_content
                                    is_readonly=is_spectator.get()
                                    force_unified=true
                                    mobile=true
                                    merge_conflict=merge_conflict
                                    on_resolve_merge_conflict=on_resolve
                                    on_close=Callback::new(move |_| set_diff_content.set(None))
                                />
                            }
                            .into_any()
                    } else if let Some(doc_id) = current_editor_doc.get() {
                        view! { <Editor doc_id=doc_id on_stats=on_stats embedded=true /> }.into_any()
                    } else {
                        view! { <Dashboard /> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
