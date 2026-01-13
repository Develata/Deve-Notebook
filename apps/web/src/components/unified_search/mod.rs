pub mod types;
pub mod providers;

use leptos::prelude::*;
use web_sys::{HtmlInputElement, KeyboardEvent, MouseEvent};
use crate::i18n::{Locale, t};
use self::types::{SearchProvider, SearchResult, SearchAction};
use self::providers::{FileProvider, CommandProvider};
use crate::components::command_palette::commands::create_static_commands;
use crate::hooks::use_core::use_core;

#[component]
pub fn UnifiedSearch(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] set_show: WriteSignal<bool>,
    #[prop(into)] mode_signal: Signal<String>, // ">" or ""
    on_settings: Callback<()>,
    on_open: Callback<()>, // Legacy open callback or other action
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let core = use_core();
    
    let (query, set_query) = signal(String::new());
    let (selected_index, set_selected_index) = signal(0);
    
    // Reset state when shown
    Effect::new(move |_| {
        if show.get() {
            set_query.set(mode_signal.get());
            set_selected_index.set(0);
        }
    });

    let providers_results = Memo::new(move |_| {
         if !show.get() { return Vec::new(); }

         let q = query.get();
         let current_docs = core.docs.get();
         let current_locale = locale.get();
         
         // 1. Determine active provider triggered by char
         // Priority: Command (>) -> File (Default)
         
         let is_command = q.starts_with('>');
         
         if is_command {
             let cmds = create_static_commands(
                current_locale,
                on_settings,
                on_open,
                set_show,
                locale,
             );
             let provider = CommandProvider::new(cmds);
             provider.search(&q)
         } else {
             let doc_list: Vec<(deve_core::models::DocId, String)> = current_docs.iter()
                .map(|(k, v)| (*k, v.clone()))
                .collect();
             let provider = FileProvider::new(doc_list);
             provider.search(&q)
         }
    });

    let active_index = move || {
        let count = providers_results.get().len();
        if count == 0 { return 0; }
        let current = selected_index.get();
        if current >= count { 0 } else { current }
    };

    let handle_keydown = move |ev: KeyboardEvent| {
        let key = ev.key();
        
        // Allow closing with Ctrl+K or Escape
        if (ev.ctrl_key() || ev.meta_key()) && (key == "k" || key == "p") {
            ev.prevent_default();
            ev.stop_propagation();
            set_show.set(false);
            return;
        }

        let count = providers_results.get().len();
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
                if let Some(res) = providers_results.get().get(idx) {
                    match &res.action {
                        SearchAction::OpenDoc(id) => {
                            core.on_doc_select.run(id.clone());
                            set_show.set(false);
                        }
                        SearchAction::RunCommand(cmd) => {
                            cmd.action.run(());
                            // Command action usually closes panel via set_show calling logic inside command,
                            // but we can ensure it here if command logic doesn't.
                            // Currently command logic handles it.
                        }
                    }
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
                    on:click=move |ev: MouseEvent| ev.stop_propagation()
                    on:keydown=handle_keydown
                >
                    <div class="p-3 border-b border-gray-100 flex items-center gap-3 bg-gray-50/50">
                        <svg class="w-4 h-4 text-gray-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                             {move || if query.get().starts_with('>') {
                                 // Command Icon
                                 view! { <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" /> }.into_any()
                             } else {
                                 // File Icon
                                 view! { <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" /> }.into_any()
                             }}
                        </svg>
                        <input 
                            type="text"
                            class="flex-1 outline-none text-base bg-transparent text-gray-800 placeholder:text-gray-400"
                            placeholder=move || {
                                if query.get().starts_with('>') {
                                    if locale.get() == Locale::Zh { "搜索命令..." } else { "Search commands..." }
                                } else {
                                    if locale.get() == Locale::Zh { "搜索文件..." } else { "Search files..." }
                                }
                            }
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
                            let res = providers_results.get();
                            
                            if res.is_empty() {
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
                                            each=move || res.clone().into_iter().enumerate()
                                            key=|(idx, r)| format!("{}-{}", idx, r.id)
                                            children=move |(idx, item)| {
                                                let is_sel = idx == idx_sel;
                                                let detail_icon = item.detail.clone();
                                                let detail_text = item.detail.clone();
                                                let detail_text_cond = detail_text.clone();
                                                view! {
                                                    <button
                                                        class=format!(
                                                            "w-full text-left px-4 py-3 rounded-lg flex items-center gap-3 group transition-colors {}",
                                                            if is_sel { "bg-blue-50 text-blue-700" } else { "text-gray-700 hover:bg-gray-50" }
                                                        )
                                                        on:click=move |_| {
                                                            match &item.action {
                                                                SearchAction::OpenDoc(id) => {
                                                                    core.on_doc_select.run(id.clone());
                                                                    set_show.set(false);
                                                                }
                                                                SearchAction::RunCommand(cmd) => {
                                                                    cmd.action.run(());
                                                                }
                                                            }
                                                        }
                                                        on:mousemove=move |_| set_selected_index.set(idx)
                                                    >
                                                        <div class=format!("flex-none {}", if is_sel { "text-blue-500" } else { "text-gray-400" })>
                                                            <Show when=move || detail_icon.as_deref() == Some("Command") fallback=|| view! {
                                                                <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                                     <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21h10a2 2 0 002-2V9.414a1 1 0 00-.293-.707l-5.414-5.414A1 1 0 0012.586 3H7a2 2 0 00-2 2v14a2 2 0 002 2z" />
                                                                </svg>
                                                            }>
                                                                <svg class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                                                     <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
                                                                </svg>
                                                            </Show>
                                                        </div>

                                                        <div class="flex-1 truncate flex flex-col items-start gap-0.5">
                                                            <span class="font-medium">{item.title.clone()}</span>
                                                            <Show when=move || detail_text_cond.is_some()>
                                                                <span class="text-xs opacity-60 font-mono">
                                                                    {detail_text.clone().unwrap()}
                                                                </span>
                                                            </Show>
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
