// apps/web/src/components/mobile_layout/drawers/right.rs

use crate::components::outline::Outline;
use crate::editor::ffi::scroll_global;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

use super::drawer_class;

#[component]
pub fn RightDrawer(
    open: ReadSignal<bool>,
    on_close: Callback<()>,
    content_signal: Option<ReadSignal<String>>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <div class=move || drawer_class("right", open.get())>
            <div class="flex flex-col h-full">
                <div
                    class="h-12 px-3 flex items-center justify-between border-b border-gray-200 text-sm font-semibold"
                    style="padding-top: env(safe-area-inset-top);"
                >
                    <span class="text-gray-800 flex items-center gap-1">
                        {move || t::sidebar::outline(locale.get())}
                    </span>
                    <button
                        class="h-11 min-w-11 px-3 text-sm font-medium text-gray-600 rounded-md hover:bg-gray-100 active:bg-gray-200 transition-colors duration-200 ease-out"
                        title=move || t::sidebar::close_outline(locale.get())
                        aria-label=move || t::sidebar::close_outline(locale.get())
                        on:click=move |_| on_close.run(())
                    >
                        {move || t::sidebar::close_outline(locale.get())}
                    </button>
                </div>

                <div
                    class="flex-1 overflow-y-auto px-2 pb-3"
                    style="padding-bottom: env(safe-area-inset-bottom);"
                >
                    {move || {
                        if let Some(content) = content_signal {
                            view! {
                                <Outline
                                    content=content
                                    on_scroll=Callback::new(move |line| {
                                        let close = on_close.clone();
                                        request_animation_frame(move || {
                                            scroll_global(line);
                                            close.run(());
                                        });
                                    })
                                />
                            }
                            .into_any()
                        } else {
                            view! {
                                <div class="px-4 py-8 text-sm text-gray-500 flex flex-col items-center gap-2">
                                    <div class="w-9 h-9 rounded-full bg-gray-100 flex items-center justify-center text-gray-400">
                                        "∅"
                                    </div>
                                    <div class="text-gray-600">{move || t::sidebar::outline_unavailable(locale.get())}</div>
                                    <div class="text-[11px] text-gray-400">{move || t::sidebar::no_headings_found(locale.get())}</div>
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
