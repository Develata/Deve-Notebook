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
    
    // Global Locale
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    // Open Doc By Path State
    let (show_open_modal, set_show_open_modal) = signal(false);

    // Global Key Handler
    let handle_keydown = move |ev: KeyboardEvent| {
        let is_ctrl = ev.meta_key() || ev.ctrl_key();
        let key = ev.key().to_lowercase();
        
        if is_ctrl && key == "k" {
            ev.prevent_default();
            ev.stop_propagation(); 
            set_show_cmd.update(|s| *s = !*s);
        }
        
        // Ctrl+L: Toggle Language
        if is_ctrl && key == "l" {
             ev.prevent_default();
             ev.stop_propagation();
             locale.update(|l| *l = l.toggle());
        }

        // Ctrl+O: Open Document Modal
        if is_ctrl && key == "o" {
             ev.prevent_default();
             ev.stop_propagation();
             set_show_open_modal.set(true);
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
         set_show_open_modal.set(true);
    });

    let on_open_confirm = Callback::new(move |path: String| {
        let normalized = path.replace('\\', "/");
        // Auto-append .md if missing (assuming user might omit it)
        let target = if normalized.ends_with(".md") { normalized.clone() } else { format!("{}.md", normalized) };
        
        // Find doc
        let list = docs.get_untracked();
        // Exact match first, then permissive
        if let Some((id, _)) = list.iter().find(|(_, p)| p == &target || p == &normalized) {
            set_current_doc.set(Some(*id));
        } else {
            leptos::logging::warn!("Document not found: {}", target);
            // TODO: Toast "Not Found"
        }
    });

    // Create Doc Callback
    let ws_for_create = ws.clone();
    let on_doc_create = Callback::new(move |name: String| {
        ws_for_create.send(ClientMessage::CreateDoc { name });
    });

    // Rename Doc Callback
    let ws_for_rename = ws.clone();
    let on_doc_rename = Callback::new(move |(old_path, new_path): (String, String)| {
        leptos::logging::log!("App: on_doc_rename sending msg: old={} new={}", old_path, new_path);
        ws_for_rename.send(ClientMessage::RenameDoc { old_path, new_path });
    });

    // Delete Doc Callback
    let ws_for_delete = ws.clone();
    let on_doc_delete = Callback::new(move |path: String| {
        ws_for_delete.send(ClientMessage::DeleteDoc { path });
    });

    // Layout State
    let (sidebar_width, set_sidebar_width) = signal(250); // Default width
    let (is_resizing, set_is_resizing) = signal(false);
    
    let start_resize = Callback::new(move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        set_is_resizing.set(true);
    });
    
    let stop_resize = Callback::new(move |_| {
        set_is_resizing.set(false);
    });
    
    let do_resize = Callback::new(move |ev: web_sys::MouseEvent| {
        if is_resizing.get_untracked() {
            let new_width = ev.client_x();
            // Clamp width (min 150, max 600)
            if new_width > 150 && new_width < 600 {
                set_sidebar_width.set(new_width);
            }
        }
    });

    view! {
        <div 
            class="h-screen w-screen flex flex-col bg-gray-50 text-gray-900 font-sans"
            on:keydown=handle_keydown
            on:mousemove=move |ev| do_resize.run(ev)
            on:mouseup=move |_| stop_resize.run(())
            on:mouseleave=move |_| stop_resize.run(())
            tabindex="-1" 
            style=move || if is_resizing.get() { "cursor: col-resize; user-select: none;" } else { "" }
        >
            <crate::components::command_palette::CommandPalette 
                show=show_cmd 
                set_show=set_show_cmd
                docs=docs
                on_select_doc=on_doc_select
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

            // Re-use InputModal for "Open Document"
            <crate::components::input_modal::InputModal
                show=show_open_modal
                set_show=set_show_open_modal
                title="Open Document"
                placeholder="folder/file.md or folder\\file"
                confirm_label="Open"
                initial_value={None::<String>}
                on_confirm=on_open_confirm
            />

            <main class="flex-1 w-full max-w-[1400px] mx-auto p-4 flex overflow-hidden">
                 // Left Sidebar
                 <aside 
                    class="flex-none bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden"
                    style=move || format!("width: {}px", sidebar_width.get())
                 >
                     <crate::components::sidebar::Sidebar 
                        docs=docs
                        current_doc=current_doc
                        on_select=on_doc_select
                        on_create=on_doc_create
                        on_rename=on_doc_rename
                        on_delete=on_doc_delete
                     />
                 </aside>
                 
                 // Drag Handle
                 <div 
                    class="w-4 flex-none cursor-col-resize flex items-center justify-center hover:bg-blue-50/50 group transition-colors"
                    on:mousedown=move |ev| start_resize.run(ev)
                 >
                    // Visual indicator (thin line)
                    <div class="w-[1px] h-8 bg-gray-200 group-hover:bg-blue-300 transition-colors"></div>
                 </div>

                 // Main Editor
                 <div class="flex-1 bg-white shadow-sm border border-gray-200 rounded-lg overflow-hidden relative flex flex-col min-w-0">
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
