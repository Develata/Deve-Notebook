// apps\web\src\components
//! # MergeModal 组件 (MergeModal Component)
//!
//! 手动合并模式下用于审核和合并待处理操作的模态对话框。
//! 从底部状态栏或分支切换器触发。

use leptos::prelude::*;
use crate::hooks::use_core::CoreState;

#[component]
pub fn MergeModal(
    show: ReadSignal<bool>,
    set_show: WriteSignal<bool>,
) -> impl IntoView {
    let core = expect_context::<CoreState>();
    
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
                <div class="bg-white rounded-xl shadow-2xl w-full max-w-lg p-6 flex flex-col max-h-[80vh]">
                    <div class="flex items-center justify-between mb-4 flex-none">
                        <h2 class="text-xl font-bold text-gray-800">"Pending Merges"</h2>
                        <button 
                            class="p-1 hover:bg-gray-100 rounded-full text-gray-500"
                            on:click=move |_| set_show.set(false)
                        >
                            "✕"
                        </button>
                    </div>
                    
                    <div class="bg-blue-50 text-blue-800 p-3 rounded-lg text-sm mb-4 flex-none">
                        "Manual Mode is active. These changes from peers are waiting for your approval."
                    </div>
                    
                    // 预览列表
                    <div class="flex-1 overflow-y-auto mb-4 border border-gray-100 rounded-lg p-2 bg-gray-50">
                        <For
                            each=move || core.pending_ops_previews.get()
                            key=|(path, _, _)| path.clone()
                            children=move |(path, old_preview, new_preview)| {
                                view! {
                                    <div class="bg-white rounded shadow-sm border border-gray-200 p-3 mb-2 text-sm last:mb-0">
                                        <div class="font-medium text-gray-700 mb-2 border-b border-gray-100 pb-1">{path}</div>
                                        <div class="grid grid-cols-2 gap-2 font-mono text-xs">
                                            <div>
                                                <div class="text-xs text-gray-400 mb-0.5">"Current"</div>
                                                <div class="bg-red-50 text-red-700 p-1.5 rounded break-all">
                                                    {format!("- {}", old_preview)}
                                                </div>
                                            </div>
                                            <div>
                                                 <div class="text-xs text-gray-400 mb-0.5">"Incoming"</div>
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
                                <div class="text-center py-8 text-gray-400 italic">
                                    "No pending operations." 
                                </div> 
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </div>
                    
                    // 操作区
                    <div class="flex gap-3 flex-none pt-2 border-t border-gray-100">
                        <button 
                            class="flex-1 px-4 py-2 bg-gray-100 text-gray-700 font-semibold rounded-lg hover:bg-gray-200 transition-colors"
                            on:click=discard_pending
                        >
                            "Discard All"
                        </button>
                        <button 
                            class="flex-1 px-4 py-2 bg-green-600 text-white font-semibold rounded-lg hover:bg-green-700 transition-colors shadow-sm"
                            on:click=confirm_merge
                        >
                            {move || format!("Merge {} Operations", core.pending_ops_count.get())}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
