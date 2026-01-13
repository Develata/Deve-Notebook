use leptos::prelude::*;
use crate::api::ConnectionStatus;
use crate::i18n::{Locale, t};
use crate::editor::EditorStats;
use crate::components::branch_switcher::BranchSwitcher;

#[component]
pub fn BottomBar(
    status: ReadSignal<ConnectionStatus>,
    stats: ReadSignal<EditorStats>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

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

    view! {
        <footer class="h-8 bg-gray-50 border-t border-gray-200 flex items-center justify-between px-4 select-none">
            // Left: Branch Switcher + System Status
            <div class="flex items-center gap-3">
                <BranchSwitcher />
                <div class="w-px h-4 bg-gray-200"></div>
                {status_view}
            </div>

            // Right: Editor Stats
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
