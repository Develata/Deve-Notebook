use leptos::prelude::*;
use deve_core::models::DocId;

#[component]
pub fn Sidebar(
    docs: ReadSignal<Vec<(DocId, String)>>,
    current_doc: ReadSignal<Option<DocId>>,
    on_select: Callback<DocId>,
) -> impl IntoView {
    view! {
        <div class="h-full w-64 bg-gray-50 border-r border-gray-200 flex flex-col">
            <div class="p-4 font-bold text-gray-500 text-xs tracking-wider">
                "EXPLORER"
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
            </div>
        </div>
    }
}
