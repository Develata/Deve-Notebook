//! CommandPalette 组件 (CommandPalette Component)
//!
//! 一个可搜索的命令面板，用于快速执行操作（仅限命令，不包括文件搜索）。

mod types;
mod commands;

pub use types::Command;

use leptos::prelude::*;
use crate::i18n::{Locale, t};
use self::commands::{create_static_commands, filter_commands};

#[component]
pub fn CommandPalette(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] set_show: WriteSignal<bool>,
    on_settings: Callback<()>,
    on_open: Callback<()>,  // Opens the Open Document modal
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (query, set_query) = signal(String::new());
    let (selected_index, set_selected_index) = signal(0);
    
    // 显示时重置选择
    Effect::new(move |_| {
        if show.get() {
            set_query.set(String::new());
            set_selected_index.set(0);
        }
    });

    let filtered_commands = Memo::new(move |_| {
        let q = query.get();
        let current_locale = locale.get();
        
        let static_cmds = create_static_commands(
            current_locale,
            on_settings,
            on_open,
            set_show,
            locale,
        );
        
        // 仅过滤静态命令，不包含文件命令
        filter_commands(&q, static_cmds, 50)
    });

    // 验证索引
    let active_index = move || {
        let count = filtered_commands.get().len();
        if count == 0 { return 0; }
        let current = selected_index.get();
        if current >= count { 0 } else { current }
    };

    // 键盘导航
    let handle_keydown = move |ev: web_sys::KeyboardEvent| {
        let key = ev.key();
        if (ev.ctrl_key() || ev.meta_key()) && key == "k" {
            ev.prevent_default();
            ev.stop_propagation();
            set_show.set(false);
            return;
        }

        let count = filtered_commands.get().len();
        if count == 0 { return; }
        
        match key.as_str() {
            "ArrowDown" => {
                ev.prevent_default();
                set_selected_index.update(|i| *i = (*i + 1) % count);
            }
            "ArrowUp" => {
                ev.prevent_default();
                set_selected_index.update(|i| *i = (*i + count - 1) % count);
            }
            "Enter" => {
                ev.prevent_default();
                let idx = active_index();
                if let Some(cmd) = filtered_commands.get().get(idx) {
                    cmd.action.run(());
                }
            }
            _ => {}
        }
    };

    view! {
        <Show when=move || show.get()>
            <div 
                class="fixed inset-0 z-[60] font-sans"
                on:click=move |_| set_show.set(false)
            >
                <div 
                    class="absolute top-2 left-1/2 -translate-x-1/2 w-full max-w-xl bg-white rounded-lg shadow-xl border border-gray-200 overflow-hidden flex flex-col max-h-[60vh] animate-in fade-in zoom-in-95 duration-100"
                    on:click=move |ev| ev.stop_propagation()
                    on:keydown=handle_keydown
                >
                    <div class="p-3 border-b border-gray-100 flex items-center gap-3 bg-gray-50/50">
                        <svg class="w-4 h-4 text-gray-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>
                        <input 
                            type="text"
                            class="flex-1 outline-none text-base bg-transparent text-gray-800 placeholder:text-gray-400"
                            placeholder=move || if locale.get() == Locale::Zh { "输入命令..." } else { "Type a command..." }
                            prop:value=move || query.get()
                            on:input=move |ev| {
                                set_query.set(event_target_value(&ev));
                                set_selected_index.set(0);
                            }
                            autofocus
                        />
                    </div>
                    
                    <div class="overflow-y-auto p-2">
                        {move || {
                            let cmds = filtered_commands.get();
                            
                            if cmds.is_empty() {
                                view! {
                                    <div class="p-4 text-center text-gray-400 text-sm">
                                        {move || t::command_palette::no_results(locale.get())}
                                    </div>
                                }.into_any()
                            } else {
                                let idx_sel = active_index();
                                view! {
                                    <div class="flex flex-col gap-1">
                                        <For
                                            each=move || cmds.clone().into_iter().enumerate()
                                            key=|(_, cmd)| cmd.id.clone()
                                            children=move |(idx, cmd)| {
                                                let is_sel = idx == idx_sel;
                                                view! {
                                                    <button
                                                        class=format!(
                                                            "w-full text-left px-4 py-3 rounded-lg flex items-center gap-3 group transition-colors {}",
                                                            if is_sel { "bg-blue-50 text-blue-700" } else { "text-gray-700 hover:bg-gray-50" }
                                                        )
                                                        on:click=move |_| cmd.action.run(())
                                                        on:mousemove=move |_| set_selected_index.set(idx)
                                                    >
                                                        <div class=format!("flex-none {}", if is_sel { "text-blue-500" } else { "text-gray-400" })>
                                                            <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                                 <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                                                            </svg>
                                                        </div>

                                                        <div class="flex-1 truncate">
                                                            <span class="font-medium">{cmd.title.clone()}</span>
                                                        </div>

                                                        <Show when=move || is_sel>
                                                            <svg class="w-4 h-4 text-blue-500 opacity-0 group-hover:opacity-100 transition-opacity" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6" />
                                                            </svg>
                                                        </Show>
                                                    </button>
                                                }
                                            }
                                        />
                                    </div>
                                }.into_any()
                            }
                        }}
                    </div>
                     <div class="bg-gray-50 px-4 py-2 border-t border-gray-100 flex justify-between items-center text-xs text-gray-500">
                        <div class="flex gap-4">
                            <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Up/Down</kbd> to navigate</span>
                            <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Enter</kbd> to select</span>
                        </div>
                        <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Esc</kbd> to close</span>
                    </div>
                </div>
            </div>
        </Show>
    }
}
