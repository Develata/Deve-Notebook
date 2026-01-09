use leptos::prelude::*;
use crate::i18n::{Locale, t};

#[derive(Clone, Debug)]
pub struct Command {
    pub id: &'static str,
    pub title_key: fn(Locale) -> &'static str,
    pub action: Callback<()>,
}

#[component]
pub fn CommandPalette(
    #[prop(into)] show: Signal<bool>,
    #[prop(into)] set_show: WriteSignal<bool>,
    on_settings: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    // Removed internal show signal
    let (query, set_query) = signal(String::new());
    let (selected_index, set_selected_index) = signal(0);
    
    // Command Registry
    let commands = move || {
        vec![
            Command {
                id: "settings", 
                title_key: t::command_palette::open_settings,
                action: Callback::new(move |_| {
                    request_animation_frame(move || {
                        on_settings.run(());
                        set_show.set(false);
                    });
                })
            },
            Command {
                id: "lang",
                title_key: t::command_palette::toggle_language,
                action: Callback::new(move |_| {
                    request_animation_frame(move || {
                        locale.update(|l| *l = l.toggle());
                        set_show.set(false);
                    });
                })
            }
        ]
    };

    // Reset selection when shown
    Effect::new(move |_| {
        if show.get() {
            set_query.set(String::new());
            set_selected_index.set(0);
        }
    });

    view! {
        <Show when=move || show.get()>
            // Transparent overlay for click-outside dismissal
            <div 
                class="fixed inset-0 z-[60]"
                on:click=move |_| set_show.set(false)
            >
                // The Palette Box - Top Center Floating
                <div 
                    class="absolute top-2 left-1/2 -translate-x-1/2 w-full max-w-xl bg-white rounded-lg shadow-xl border border-gray-200 overflow-hidden flex flex-col max-h-[60vh] animate-in fade-in zoom-in-95 duration-100"
                    on:click=move |ev| ev.stop_propagation() // Prevent closing when clicking inside
                >
                    <div class="p-3 border-b border-gray-100 flex items-center gap-3 bg-gray-50/50">
                        <svg class="w-4 h-4 text-gray-400" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
                        </svg>
                        <input 
                            type="text"
                            class="flex-1 outline-none text-base bg-transparent text-gray-800 placeholder:text-gray-400"
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
                            let filtered: Vec<_> = cmds.into_iter().filter(|cmd| {
                                (cmd.title_key)(locale.get()).to_lowercase().contains(&q)
                            }).collect();
                            
                            if filtered.is_empty() {
                                view! {
                                    <div class="p-4 text-center text-gray-400 text-sm">
                                        {move || t::command_palette::no_results(locale.get())}
                                    </div>
                                }.into_any()
                            } else {
                                let count = filtered.len();
                                if selected_index.get() >= count {
                                    set_selected_index.set(0); 
                                }
                                
                                view! {
                                    <div class="flex flex-col gap-1">
                                        <For
                                            each=move || filtered.clone().into_iter().enumerate()
                                            key=|(_, cmd)| cmd.id
                                            children=move |(idx, cmd)| {
                                                let is_sel = idx == selected_index.get();
                                                view! {
                                                    <button
                                                        class=format!(
                                                            "w-full text-left px-4 py-3 rounded-lg flex items-center justify-between group transition-colors {}",
                                                            if is_sel { "bg-blue-50 text-blue-700" } else { "text-gray-700 hover:bg-gray-50" }
                                                        )
                                                        on:click=move |_| cmd.action.run(())
                                                    >
                                                        <span class="font-medium">{(cmd.title_key)(locale.get())}</span>
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
