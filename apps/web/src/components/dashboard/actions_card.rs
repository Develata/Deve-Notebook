// apps/web/src/components/dashboard/actions_card.rs
//! # Actions Card (快捷操作卡片)
//!
//! 提供 "New Doc" 和 "Sync Now" 按钮。

use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

#[component]
pub fn ActionsCard() -> impl IntoView {
    let core = expect_context::<CoreState>();

    let on_new_doc = move |_| {
        core.on_doc_create.run("Untitled.md".to_string());
    };

    let on_sync = move |_| {
        core.on_get_sync_mode.run(());
    };

    view! {
        <div class="bg-panel rounded-lg border border-default p-4">
            <h3 class="text-sm font-semibold text-secondary mb-3">"Quick Actions"</h3>
            <div class="flex gap-2">
                <button
                    class="flex-1 px-3 py-2 text-xs font-medium rounded-md \
                           bg-accent text-on-accent hover:bg-accent/90 transition-colors"
                    on:click=on_new_doc
                >
                    "New Doc"
                </button>
                <button
                    class="flex-1 px-3 py-2 text-xs font-medium rounded-md \
                           border border-default text-primary hover:bg-active transition-colors"
                    on:click=on_sync
                >
                    "Sync Now"
                </button>
            </div>
        </div>
    }
}
