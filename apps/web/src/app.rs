use crate::editor::Editor;
use crate::api::WsService;
use leptos::prelude::*;
use deve_core::models::DocId;
use deve_core::protocol::{ClientMessage, ServerMessage};
use crate::i18n::Locale;
use web_sys::KeyboardEvent;

#[component]
pub fn App() -> impl IntoView {
    // Global Locale State
    let locale = RwSignal::new(Locale::default());
    provide_context(locale);

    view! {
        <AppContent/>
    }
}

#[component]
fn AppContent() -> impl IntoView {
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

    // Stats State
    let (stats, set_stats) = signal(crate::editor::EditorStats::default());
    let on_stats = Callback::new(move |s| set_stats.set(s));

    // Command Palette State
    let (show_cmd, set_show_cmd) = signal(false);
    
    // Global Key Handler
    let handle_keydown = move |ev: KeyboardEvent| {
        if (ev.meta_key() || ev.ctrl_key()) && ev.key() == "k" {
            ev.prevent_default();
            ev.stop_propagation(); // Important to stop browser default
            set_show_cmd.update(|s| *s = !*s);
        }
        
        if show_cmd.get_untracked() && ev.key() == "Escape" {
             set_show_cmd.set(false);
        }
    };

    // Header Callbacks
    let on_command = Callback::new(move |_| {
        set_show_cmd.update(|s| *s = !*s);
    });
    
    let on_home = Callback::new(move |_| {
        // For now, Home deselects current doc (Show Welcome)
        set_current_doc.set(None);
    });

    let on_open = Callback::new(move |_| {
         // Logic to find "index" or similar.
         let list = docs.get_untracked();
         if let Some((id, _)) = list.iter().find(|(_, name)| name == "index" || name == "index.md" || name == "Home") {
             set_current_doc.set(Some(*id));
         } else {
             // Fallback: Just open the first one or do nothing?
         }
    });

    view! {
        <div 
            class="h-screen w-screen flex flex-col bg-gray-50 text-gray-900 font-sans"
            on:keydown=handle_keydown
            tabindex="-1" 
        >
            <crate::components::command_palette::CommandPalette 
                show=show_cmd 
                set_show=set_show_cmd
                on_settings=on_settings 
            />
            <crate::components::header::Header 
                status_text=status_text 
                on_home=on_home
                on_open=on_open
                on_command=on_command
            />
            
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
                             <Editor doc_id=id on_stats=on_stats /> 
                        }.into_any(),
                        None => view! { 
                            <div class="flex items-center justify-center h-full text-gray-400">
                                "Select a document to edit"
                            </div> 
                        }.into_any()
                    }}
                 </div>
            </main>
            
            <crate::components::bottom_bar::BottomBar status=ws.status stats=stats />
        </div>
    }
}
