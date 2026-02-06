// apps/web/src/components/mobile_layout/drawers/left.rs

use crate::components::activity_bar::SidebarView;
use crate::components::sidebar::Sidebar;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

use super::drawer_class;

#[component]
pub fn LeftDrawer(
    core: CoreState,
    active_view: ReadSignal<SidebarView>,
    open: ReadSignal<bool>,
    on_doc_select: Callback<deve_core::models::DocId>,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <div class=move || drawer_class("left", open.get())>
            <div class="flex flex-col h-full">
                <div
                    class="h-12 px-3 flex items-center justify-between border-b border-gray-200 text-sm font-semibold"
                    style="padding-top: env(safe-area-inset-top);"
                >
                    <span class="text-gray-800 flex items-center gap-1">
                        {move || t::sidebar::files(locale.get())}
                    </span>
                    <button
                        class="h-11 min-w-11 px-3 text-sm font-medium text-gray-600 rounded-md hover:bg-gray-100 active:bg-gray-200 transition-colors duration-200 ease-out"
                        title=move || t::sidebar::close_file_tree(locale.get())
                        aria-label=move || t::sidebar::close_file_tree(locale.get())
                        on:click=move |_| on_close.run(())
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="2" class="w-4 h-4 mx-auto">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M6 6l8 8M14 6l-8 8" />
                        </svg>
                    </button>
                </div>

                <div
                    class="flex-1 overflow-hidden px-2 pb-3"
                    style="padding-bottom: env(safe-area-inset-bottom);"
                >
                    {move || {
                        let docs = core.docs.get();
                        if docs.is_empty() {
                            view! {
                                <div class="px-4 py-8 text-sm text-gray-500 flex flex-col items-center gap-3">
                                    <div class="w-9 h-9 rounded-full bg-gray-100 flex items-center justify-center text-gray-400">
                                        "∅"
                                    </div>
                                    <div class="text-gray-600">{move || t::sidebar::no_docs_yet(locale.get())}</div>
                                    <div class="text-[11px] text-gray-400">{move || t::sidebar::create_first_note(locale.get())}</div>
                                    <button
                                        class="px-3 py-1 text-xs font-medium text-blue-600 rounded-md border border-blue-100 bg-blue-50"
                                        on:click=move |_| {
                                            core.on_doc_create.run(t::sidebar::new_note(locale.get()).to_string());
                                            on_close.run(());
                                        }
                                    >
                                        {move || t::sidebar::new_note(locale.get())}
                                    </button>
                                </div>
                            }
                            .into_any()
                        } else {
                            view! {
                                <div class="h-full overflow-y-auto">
                                    <Sidebar
                                        active_view=active_view
                                        docs=core.docs
                                        current_doc=core.current_doc
                                        on_select=Callback::new(move |id| {
                                            on_doc_select.run(id);
                                            on_close.run(())
                                        })
                                        on_delete=core.on_doc_delete
                                    />
                                </div>
                            }
                            .into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
