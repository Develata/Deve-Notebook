//! # Merge Panel Component
//!
//! Displays sync mode toggle and pending operations for Manual Merge.

use leptos::prelude::*;

#[component]
pub fn MergePanel() -> impl IntoView {
    let core = expect_context::<crate::hooks::use_core::CoreState>();
    
    // Fetch initial state on mount
    Effect::new(move |_| {
        core.on_get_sync_mode.run(());
        core.on_get_pending_ops.run(());
    });
    
    let is_manual = Memo::new(move |_| core.sync_mode.get() == "manual");
    let has_pending = Memo::new(move |_| core.pending_ops_count.get() > 0);
    
    let toggle_mode = move |_| {
        let new_mode = if core.sync_mode.get_untracked() == "auto" {
            "manual".to_string()
        } else {
            "auto".to_string()
        };
        core.on_set_sync_mode.run(new_mode);
    };
    
    let confirm_merge = move |_| {
        core.on_confirm_merge.run(());
    };
    
    let discard_pending = move |_| {
        core.on_discard_pending.run(());
    };
    
    view! {
        <div class="p-4 bg-white border-b border-gray-200">
            <div class="flex items-center justify-between mb-4">
                <div class="flex items-center gap-2">
                    <span class="text-sm font-medium text-gray-700">"Sync Mode:"</span>
                    <button 
                        class=move || format!(
                            "px-3 py-1 text-xs font-semibold rounded-full transition-colors {}",
                            if is_manual.get() {
                                "bg-yellow-100 text-yellow-800 border border-yellow-300"
                            } else {
                                "bg-green-100 text-green-800 border border-green-300"
                            }
                        )
                        on:click=toggle_mode
                    >
                        {move || if is_manual.get() { "Manual" } else { "Auto" }}
                    </button>
                </div>
                
                {move || if has_pending.get() {
                    view! {
                        <div class="flex items-center gap-2 px-3 py-1 bg-orange-100 text-orange-800 rounded-full text-xs font-semibold">
                            <span class="w-2 h-2 bg-orange-500 rounded-full animate-pulse"></span>
                            {move || format!("{} pending", core.pending_ops_count.get())}
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
            
            {move || if is_manual.get() && has_pending.get() {
                view! {
                    <div class="bg-gray-50 rounded-lg border border-gray-200 p-4">
                        <h3 class="text-sm font-semibold text-gray-700 mb-3">"Pending Operations"</h3>
                        
                        <div class="space-y-2 mb-4 max-h-48 overflow-y-auto">
                            <For
                                each=move || core.pending_ops_previews.get()
                                key=|(path, _, _)| path.clone()
                                children=move |(path, old_preview, new_preview)| {
                                    view! {
                                        <div class="bg-white rounded border border-gray-100 p-2 text-xs">
                                            <div class="font-medium text-gray-700 mb-1">{path}</div>
                                            <div class="grid grid-cols-2 gap-2">
                                                <div class="bg-red-50 p-1 rounded text-red-700 font-mono truncate">
                                                    {format!("- {}", old_preview)}
                                                </div>
                                                <div class="bg-green-50 p-1 rounded text-green-700 font-mono truncate">
                                                    {format!("+ {}", new_preview)}
                                                </div>
                                            </div>
                                        </div>
                                    }
                                }
                            />
                        </div>
                        
                        <div class="flex gap-2">
                            <button 
                                class="flex-1 px-4 py-2 bg-green-600 text-white text-sm font-semibold rounded-lg hover:bg-green-700 transition-colors"
                                on:click=confirm_merge
                            >
                                "Confirm Merge"
                            </button>
                            <button 
                                class="flex-1 px-4 py-2 bg-gray-200 text-gray-700 text-sm font-semibold rounded-lg hover:bg-gray-300 transition-colors"
                                on:click=discard_pending
                            >
                                "Discard"
                            </button>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
