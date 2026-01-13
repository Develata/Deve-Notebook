//! # BottomBar 组件 (BottomBar Component)
//!
//! 底部状态栏，显示分支切换器、连接状态和编辑器统计信息 (字数、行数、字符数)。

use leptos::prelude::*;
use crate::api::ConnectionStatus;
use crate::i18n::{Locale, t};
use crate::editor::EditorStats;
use crate::components::branch_switcher::BranchSwitcher;
use crate::hooks::use_core::CoreState;

#[component]
pub fn BottomBar(
    status: ReadSignal<ConnectionStatus>,
    stats: ReadSignal<EditorStats>,
) -> impl IntoView {
    let core = expect_context::<CoreState>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let max_ver = core.doc_version;
    let curr_ver = core.playback_version;
    let set_ver = core.set_playback_version;

    let status_view = move || {
        let (color, text) = match status.get() {
            ConnectionStatus::Connected => ("bg-green-500", t::bottom_bar::ready(locale.get())),
            ConnectionStatus::Connecting => ("bg-yellow-500", t::bottom_bar::syncing(locale.get())),
            ConnectionStatus::Disconnected => ("bg-red-500", t::bottom_bar::offline(locale.get())),
        };
        
        view! {
             <div class="flex items-center gap-2">
                <div class={format!("w-2 h-2 rounded-full {}", color)}></div>
                <span class="text-xs text-gray-600 font-medium">{text}</span>
            </div>
        }
    };

    let time_travel_view = move || {
        view! {
            <div class="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 flex items-center gap-2">
                <span class="text-[10px] text-gray-500 font-mono">
                    {move || format!("v{}/{}", curr_ver.get(), max_ver.get())}
                </span>
                <input 
                    type="range" 
                    min="0" 
                    max=move || max_ver.get().to_string()
                    value=move || curr_ver.get().to_string()
                    on:input=move |ev| {
                        let val = event_target_value(&ev).parse::<u64>().unwrap_or(0);
                        set_ver.set(val);
                    }
                    class="w-32 h-1 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-[#007fd4]"
                    title="Time Travel"
                />
            </div>
        }
    };

    view! {
        <footer class="h-8 bg-gray-50 border-t border-gray-200 flex items-center justify-between px-4 select-none relative">
            // 左侧: 分支切换器 + 系统状态
            <div class="flex items-center gap-3">
                <BranchSwitcher />
                <div class="w-px h-4 bg-gray-200"></div>
                {status_view}
            </div>

            // 中间: Time Travel
            {time_travel_view}

            // 右侧: 编辑器统计

            <div class="flex items-center gap-4 text-xs text-gray-500">
                <div class="flex gap-1">
                    <span>{move || t::bottom_bar::words(locale.get())}</span>
                    <span class="font-mono text-gray-700">{move || stats.get().words}</span>
                </div>
                <div class="flex gap-1">
                    <span>{move || t::bottom_bar::lines(locale.get())}</span>
                    <span class="font-mono text-gray-700">{move || stats.get().lines}</span>
                </div>
                <div class="flex gap-1">
                    <span>{move || t::bottom_bar::col(locale.get())}</span>
                    <span class="font-mono text-gray-700">{move || stats.get().chars}</span>
                </div>
            </div>
        </footer>
    }
}
