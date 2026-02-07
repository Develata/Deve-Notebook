// apps/web/src/components/mobile_layout/content.rs
//! # Mobile Content

use crate::editor::Editor;
use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

#[component]
pub fn MobileContent(core: CoreState, drawer_open: Signal<bool>) -> impl IntoView {
    view! {
        <div
            class="relative flex-1 overflow-hidden transition-opacity flex flex-col"
            class:pointer-events-none=move || drawer_open.get()
            class:opacity-80=move || drawer_open.get()
        >
            <Show when=move || core.is_spectator.get()>
                <div class="h-6 px-3 flex items-center text-[11px] font-medium text-orange-900 bg-orange-200 border-b border-orange-300">
                    "Read-Only Mode"
                </div>
            </Show>
            <div class="flex-1 overflow-hidden">
                {move || {
                    if let Some((path, old, new)) = core.diff_content.get() {
                        return view! {
                            <crate::components::diff_view::DiffView
                                path=path
                                old_content=old
                                new_content=new
                                is_readonly=core.is_spectator.get()
                                force_unified=true
                                mobile=true
                                on_close=move || core.set_diff_content.set(None)
                            />
                        }
                        .into_any();
                    }

                    match core.current_doc.get() {
                        Some(id) => {
                            view! { <Editor doc_id=id on_stats=core.on_stats embedded=true /> }
                                .into_any()
                        }
                        None => view! {
                            <div class="flex items-center justify-center h-full text-gray-400">
                                "Select a document"
                            </div>
                        }
                        .into_any(),
                    }
                }}
            </div>
        </div>
    }
}
