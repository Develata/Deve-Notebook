// apps\web\src\components
//! # MergeModal 组件 (MergeModal Component)
//!
//! 手动合并模式下用于审核和合并待处理操作的模态对话框。
//! 从底部状态栏或分支切换器触发。

use crate::hooks::use_core::SyncMergeContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn MergeModal(show: ReadSignal<bool>, set_show: WriteSignal<bool>) -> impl IntoView {
    let core = expect_context::<SyncMergeContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    let confirm_merge = move |_| {
        core.on_confirm_merge.run(());
        set_show.set(false);
    };

    let discard_pending = move |_| {
        core.on_discard_pending.run(());
        set_show.set(false);
    };

    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 backdrop-blur-sm">
                <div class="bg-panel rounded-xl shadow-2xl w-full max-w-lg p-6 flex flex-col max-h-[80vh]">
                    <div class="flex items-center justify-between mb-4 flex-none">
                        <h2 class="text-xl font-bold text-primary">{move || t::merge::pending_merges(locale.get())}</h2>
                        <button
                            class="p-1 hover:bg-hover rounded-full text-muted"
                            on:click=move |_| set_show.set(false)
                        >
                            "✕"
                        </button>
                    </div>

                    <div class="bg-accent-subtle text-accent p-3 rounded-lg text-sm mb-4 flex-none">
                        {move || t::merge::manual_mode_hint(locale.get())}
                    </div>

                    // 预览列表
                    <div class="flex-1 overflow-y-auto mb-4 border border-default rounded-lg p-2 bg-sidebar">
                        <For
                            each=move || core.pending_ops_previews.get()
                            key=|(path, _, _)| path.clone()
                            children=move |(path, old_preview, new_preview)| {
                                view! {
                                    <div class="bg-panel rounded shadow-sm border border-default p-3 mb-2 text-sm last:mb-0">
                                        <div class="font-medium text-primary mb-2 border-b border-default pb-1">{path}</div>
                                        <div class="grid grid-cols-2 gap-2 font-mono text-xs">
                                            <div>
                                                <div class="text-xs text-muted mb-0.5">{move || t::merge::current(locale.get())}</div>
                                                <div class="bg-red-50 text-red-700 p-1.5 rounded break-all">
                                                    {format!("- {}", old_preview)}
                                                </div>
                                            </div>
                                            <div>
                                                 <div class="text-xs text-muted mb-0.5">{move || t::merge::incoming(locale.get())}</div>
                                                <div class="bg-green-50 text-green-700 p-1.5 rounded break-all">
                                                    {format!("+ {}", new_preview)}
                                                </div>
                                            </div>
                                        </div>
                                    </div>
                                }
                            }
                        />
                        {move || if core.pending_ops_previews.get().is_empty() {
                            view! {
                                <div class="text-center py-8 text-muted italic">
                                    {move || t::merge::no_pending(locale.get())}
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </div>

                    // 操作区
                    <div class="flex gap-3 flex-none pt-2 border-t border-default">
                        <button
                            class="flex-1 px-4 py-2 bg-active text-primary font-semibold rounded-lg hover:bg-hover transition-colors"
                            on:click=discard_pending
                        >
                            {move || t::merge::discard_all(locale.get())}
                        </button>
                        <button
                            class="flex-1 px-4 py-2 bg-green-600 text-white font-semibold rounded-lg hover:bg-green-700 transition-colors shadow-sm"
                            on:click=confirm_merge
                        >
                            {move || t::merge::merge_n_ops(locale.get(), core.pending_ops_count.get())}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
