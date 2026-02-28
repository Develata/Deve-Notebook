// apps\web\src\components\source_control
//! # History Component (历史记录组件)
//!
//! VS Code 风格: Timeline 视图。
//! 左侧带有连接线和圆点。

use crate::components::icons::*;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn History(expanded: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    Effect::new(move |_| {
        core.on_get_history.run(20);
    });

    view! {
        <div class="border-t border-default">
            <button
                class="w-full flex items-center px-1 py-0.5 hover:bg-hover text-[11px] font-bold text-primary uppercase"
                on:click=move |_| expanded.update(|b| *b = !*b)
            >
                <span class=move || if expanded.get() { "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform" } else { "w-4 h-4 flex items-center justify-center transition-transform" }>
                    <ChevronRight class="w-3 h-3" />
                </span>
                {move || t::source_control::graph(locale.get())}
            </button>

            {move || if expanded.get() {
                view! {
                    <div class="pb-2">
                        // Timeline List
                        <div class="relative pl-6 pt-2">
                            // Vertical Line
                            <div class="absolute left-[19px] top-2 bottom-0 w-[1px] bg-active"></div>

                            <For
                                each=move || core.commit_history.get()
                                key=|c| c.id.clone()
                                children=move |commit| {
                                    view! {
                                        <div class="relative mb-3 group">
                                            // Dot
                                            <div class="absolute -left-[19px] top-[3px] w-2.5 h-2.5 rounded-full border-2 border-white bg-accent shadow-sm z-10"></div>

                                            <div class="pr-2">
                                                <div class="text-[13px] text-primary leading-tight mb-0.5 font-medium truncate" title={commit.message.clone()}>
                                                    {commit.message.clone()}
                                                </div>
                                                <div class="flex items-center gap-2 text-[11px] text-muted">
                                                    <span class="font-mono bg-hover px-1 rounded text-secondary">{commit.id[0..7].to_string()}</span>
                                                    <span>{commit.timestamp}</span> // TODO: Format relative time
                                                </div>
                                            </div>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
