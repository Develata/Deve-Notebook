use crate::editor::Editor;
use crate::api::WsService;
use leptos::prelude::*;
use deve_core::models::DocId;
use deve_core::protocol::{ClientMessage, ServerMessage};

#[component]
pub fn App() -> impl IntoView {
    let ws = WsService::new();
    provide_context(ws.clone());
    let status_text = Signal::derive(move || format!("{:?}", ws.status.get()));

    // Global State
    let (docs, set_docs) = signal(Vec::<(DocId, String)>::new());
    let (current_doc, set_current_doc) = signal(None::<DocId>);

    // Initial List Request
    let ws_clone = ws.clone();
    Effect::new(move |_| {
         ws_clone.send(ClientMessage::ListDocs);
    });

    // Handle Messages
    Effect::new(move |_| {
        if let Some(msg) = ws.msg.get() {
            if let ServerMessage::DocList { docs: list } = msg {
                set_docs.set(list.clone());
                // Auto-select first if none selected
                if current_doc.get_untracked().is_none() {
                    if let Some(first) = list.first() {
                        set_current_doc.set(Some(first.0));
                    }
                }
            }
        }
    });

    // Callback for Sidebar
    let on_doc_select = Callback::new(move |id: DocId| {
        set_current_doc.set(Some(id));
    });

    // Settings State
    let (show_settings, set_show_settings) = signal(false);
    let on_settings = Callback::new(move |_| set_show_settings.set(true));

    view! {
        <div class="h-screen w-screen flex flex-col bg-gray-50">
            <crate::components::header::Header status_text=status_text on_settings=on_settings />
            
            <crate::components::settings::SettingsModal 
                show=show_settings 
                set_show=set_show_settings
            />

            <main class="flex-1 w-full max-w-[1400px] mx-auto p-4 flex gap-4 overflow-hidden">
                 // Left Sidebar
                 <aside class="w-64 flex-none bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden">
                     <crate::components::sidebar::Sidebar 
                        docs=docs
                        current_doc=current_doc
                        on_select=on_doc_select
                     />
                 </aside>

                 // Main Editor
                 <div class="flex-1 bg-white shadow-sm border border-gray-200 rounded-lg overflow-hidden relative flex flex-col">
                    {move || match current_doc.get() {
                        Some(id) => view! { 
                            // Keyed by ID to force re-mount on change
                             <Editor doc_id=id /> 
                        }.into_any(),
                        None => view! { 
                            <div class="flex items-center justify-center h-full text-gray-400">
                                "Select a document to edit"
                            </div> 
                        }.into_any()
                    }}
                 </div>
            </main>
        </div>
    }
}
