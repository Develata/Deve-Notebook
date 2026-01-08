use leptos::prelude::*;
use crate::i18n::{Locale, t};
use web_sys::KeyboardEvent;

#[derive(Clone, Debug)]
pub struct Command {
    pub id: &'static str,
    pub title_key: fn(Locale) -> &'static str,
    pub action: fn(), // Simple action for now, might need more context later
}

#[component]
pub fn CommandPalette(
    on_settings: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (show, set_show) = signal(false);
    let (query, set_query) = signal(String::new());
    let (selected_index, set_selected_index) = signal(0);
    
    // Command Registry
    // We construct this dynamically to allow actions like "Open Settings"
    let commands = move || {
        vec![
            (
                "settings", 
                t::command_palette::open_settings as fn(Locale) -> &'static str,
                Box::new(move || {
                    on_settings.run(());
                    set_show.set(false);
                }) as Box<dyn Fn()>
            ),
            (
                "lang",
                t::command_palette::toggle_language as fn(Locale) -> &'static str,
                Box::new(move || {
                    locale.update(|l| *l = l.toggle());
                    set_show.set(false);
                }) as Box<dyn Fn()>
            )
        ]
    };
    
    // Global Keyboard Listener
    let _handle = window_event_listener(leptos::ev::keydown, move |ev: KeyboardEvent| {
        if (ev.meta_key() || ev.ctrl_key()) && ev.key() == "k" {
            ev.prevent_default();
            set_show.update(|s| *s = !*s);
             if show.get_untracked() {
                // Focus input? (Handled by autofocus attribute hopefully)
                set_query.set(String::new());
                set_selected_index.set(0);
            }
        }
        
        if show.get() {
            if ev.key() == "Escape" {
                set_show.set(false);
            } else if ev.key() == "ArrowDown" {
                ev.prevent_default();
                set_selected_index.update(|i| *i += 1);
            } else if ev.key() == "ArrowUp" {
                ev.prevent_default();
                set_selected_index.update(|i| *i = i.saturating_sub(1));
            } else if ev.key() == "Enter" {
                ev.prevent_default();
                // Execute selected
                // (Logic needs filtered list access, handled in view or separate signal)
            }
        }
    });

    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 z-[60] flex items-start justify-center pt-[20vh] bg-black/20 backdrop-blur-sm">
                <div class="bg-white rounded-xl shadow-2xl w-full max-w-lg overflow-hidden flex flex-col max-h-[50vh]">
                    <div class="p-4 border-b border-gray-100 flex items-center gap-3">
                        <svg class="w-5 h-5 text-gray-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>
                        <input 
                            type="text"
                            class="flex-1 outline-none text-lg text-gray-800 placeholder:text-gray-400"
                            placeholder=move || t::command_palette::placeholder(locale.get())
                            prop:value=move || query.get()
                            on:input=move |ev| set_query.set(event_target_value(&ev))
                            autofocus
                        />
                    </div>
                    
                    <div class="overflow-y-auto p-2">
                        {move || {
                            let q = query.get().to_lowercase();
                            let cmds = commands();
                            let filtered: Vec<_> = cmds.into_iter().filter(|(_, title_fn, _)| {
                                title_fn(locale.get()).to_lowercase().contains(&q)
                            }).collect();
                            
                            if filtered.is_empty() {
                                view! {
                                    <div class="p-4 text-center text-gray-400 text-sm">
                                        {t::command_palette::no_results(locale.get())}
                                    </div>
                                }.into_any()
                            } else {
                                // Clamp selection
                                let count = filtered.len();
                                if selected_index.get() >= count {
                                    set_selected_index.set(0);
                                }
                                
                                // Ideally we execute here if Enter was pressed, but that's messy in render loop.
                                // Instead, we just render. Execution logic ideally shares this filtered list.
                                // For simplicity in this iteration, we won't implement Enter execution via this closure.
                                // We'll just rely on clicking for MVP or move filtering to a memo.
                                
                                view! {
                                    <div class="flex flex-col gap-1">
                                        <For
                                            each=move || filtered.clone().into_iter().enumerate()
                                            key=|(_, (id, _, _))| *id
                                            children=move |(idx, (_, title_fn, action))| {
                                                let is_sel = idx == selected_index.get();
                                                view! {
                                                    <button
                                                        class=format!(
                                                            "w-full text-left px-4 py-3 rounded-lg flex items-center justify-between group transition-colors {}",
                                                            if is_sel { "bg-blue-50 text-blue-700" } else { "text-gray-700 hover:bg-gray-50" }
                                                        )
                                                        on:click=move |_| action()
                                                    >
                                                        <span class="font-medium">{title_fn(locale.get())}</span>
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
                            <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">↑↓</kbd> to navigate</span>
                            <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Enter</kbd> to select</span>
                        </div>
                        <span><kbd class="font-sans bg-white px-1.5 py-0.5 rounded border border-gray-200">Esc</kbd> to close</span>
                    </div>
                </div>
            </div>
        </Show>
    }
}
