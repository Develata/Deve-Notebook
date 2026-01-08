use leptos::prelude::*;
use deve_core::models::DocId;
use crate::i18n::{Locale, t};

#[component]
pub fn Sidebar(
    docs: ReadSignal<Vec<(DocId, String)>>,
    current_doc: ReadSignal<Option<DocId>>,
    #[prop(into)] on_select: Callback<DocId>,
    #[prop(into)] on_create: Callback<String>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    let create_action = move |_| {
        if let Some(name) = window().prompt_with_message("Enter new file name:")
            .ok()
            .flatten() 
        {
            if !name.trim().is_empty() {
                leptos::logging::log!("Sidebar: User requested create: {}", name);
                on_create.run(name);
            }
        }
    };

    view! {
        <div class="h-full w-64 bg-gray-50 border-r border-gray-200 flex flex-col">
            <div class="p-4 flex items-center justify-between">
                <div class="font-bold text-gray-500 text-xs tracking-wider">
                    "EXPLORER"
                </div>
                <button 
                    class="text-gray-400 hover:text-gray-700 p-1 rounded hover:bg-gray-200 transition-colors"
                    title="New File"
                    on:click=create_action
                >
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                      <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                    </svg>
                </button>
            </div>
            
            <div class="flex-1 overflow-y-auto">
                <For
                    each=move || docs.get()
                    key=|doc| doc.0
                    children=move |(id, name)| {
                        // Is selected?
                        let is_selected = move || current_doc.get() == Some(id);
                        
                        view! {
                            <button
                                class=move || {
                                    let base = "w-full text-left px-4 py-2 text-sm transition-colors";
                                    if is_selected() {
                                        format!("{} bg-white border-l-2 border-blue-500 text-blue-600 font-medium", base)
                                    } else {
                                        format!("{} text-gray-600 hover:bg-gray-100 border-l-2 border-transparent", base)
                                    }
                                }
                                on:click=move |_| on_select.run(id)
                            >
                                {name}
                            </button>
                        }
                    }
                />
                <Show when=move || docs.get().is_empty()>
                    <div class="text-center text-sm text-gray-400 py-8">
                        {move || t::sidebar::no_docs(locale.get())}
                    </div>
                </Show>
            </div>
        </div>
    }
}
