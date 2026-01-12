//! # Sidebar Component (侧边栏组件)
//!
//! **架构作用**:
//! 渲染左侧文件导航栏，管理文件操作的交互流程（Context Menu, Modals）。
//!
//! **核心功能清单**:
//! - `Sidebar`: 主组件，接收全局状态与回调。
//! - `build_file_tree`: 将平铺的文档列表构建为树状结构。
//! - File Operations: 处理 Create, Rename, Copy, Move, Delete 的 UI 交互（弹窗确认）。
//! - Clipboard: 管理内部剪贴板状态。
//!
//! **类型**: Core MUST (核心必选)

use leptos::prelude::*;
use deve_core::models::DocId;

pub mod tree;
pub mod item;

use self::tree::build_file_tree;
use self::item::FileTreeItem;

#[component]
pub fn Sidebar(
    docs: ReadSignal<Vec<(DocId, String)>>,
    current_doc: ReadSignal<Option<DocId>>,
    #[prop(into)] on_select: Callback<DocId>,
    #[prop(into)] on_create: Callback<String>,
    #[prop(into)] on_rename: Callback<(String, String)>,
    #[prop(into)] on_delete: Callback<String>,
    #[prop(into)] on_copy: Callback<(String, String)>,
    #[prop(into)] on_move: Callback<(String, String)>,
) -> impl IntoView {
    
    // Create Modal State
    let (show_create, set_show_create) = signal(false);
    let (create_parent, set_create_parent) = signal(None::<String>);
    
    // Rename Modal State
    let (show_rename, set_show_rename) = signal(false);
    let (rename_target, set_rename_target) = signal(String::new());
    
    // Move Modal State
    let (show_move, set_show_move) = signal(false);
    let (move_source, set_move_source) = signal(String::new()); // The file being moved
    
    // Context Menu State
    let (active_menu, set_active_menu) = signal(None::<String>);

    // Peer List State
    let (show_peers, set_show_peers) = signal(false);
    
    // Core State (for Peers)
    let core = expect_context::<crate::hooks::use_core::CoreState>();
    
    // Clipboard State (for Copy/Paste)
    let (clipboard_path, set_clipboard_path) = signal(None::<String>);
    provide_context(clipboard_path);
    provide_context(set_clipboard_path);
    
    // Callbacks
    let request_create = Callback::new(move |parent: Option<String>| {
        set_create_parent.set(parent);
        set_show_create.set(true);
    });
    
    let confirm_create = Callback::new(move |name: String| {
        let full_path = if let Some(parent) = create_parent.get_untracked() {
             format!("{}/{}", parent, name)
         } else {
             name
         };
         on_create.run(full_path);
    });
    
    let request_rename = Callback::new(move |path: String| {
        leptos::logging::log!("Sidebar: request_rename for {}", path);
        set_rename_target.set(path);
        set_show_rename.set(true);
    });
    
    let confirm_rename = Callback::new(move |new_name: String| {
        let old = rename_target.get_untracked();
        leptos::logging::log!("Sidebar: confirm_rename called with old: {}, new: {}", old, new_name);
        // Construct new path: Parent + new_name
        // Need to parse parent from old path
        let parent = std::path::Path::new(&old).parent().and_then(|p| p.to_str()).unwrap_or("");
        // Fix backslashes for path ops if needed, but strings are usually safe here
        let parent = parent.replace("\\", "/");
        let new_path = if parent.is_empty() {
            new_name
        } else {
             format!("{}/{}", parent, new_name)
        };
        
        leptos::logging::log!("Sidebar: confirm_rename running on_rename with {}, {}", old, new_path);
        on_rename.run((old, new_path));
    });

    let request_move = Callback::new(move |path: String| {
        leptos::logging::log!("Sidebar: request_move for {}", path);
        set_move_source.set(path);
        set_show_move.set(true);
    });

    let confirm_move = Callback::new(move |dest_path: String| {
        let src = move_source.get_untracked();
        leptos::logging::log!("Sidebar: confirm_move {} -> {}", src, dest_path);
        on_move.run((src, dest_path));
    });

    let request_delete = Callback::new(move |path: String| {
         leptos::logging::log!("sidebar/mod.rs request_delete called: path={}", path);
         on_delete.run(path);
    });
    
    let on_menu_click = Callback::new(move |(path, _ev): (String, web_sys::MouseEvent)| {
        // Toggle if same, else set
        set_active_menu.update(|curr| {
            if *curr == Some(path.clone()) {
                *curr = None;
            } else {
                *curr = Some(path);
            }
        });
    });
    
    // Defer the closing to avoid destroying the component while it's processing an event
    let close_menu = Callback::new(move |_| {
         leptos::logging::log!("Sidebar: close_menu called (deferred)");
         let set_active = set_active_menu.clone();
         request_animation_frame(move || {
             set_active.set(None);
         });
    });

    let tree_nodes = Memo::new(move |_| {
        build_file_tree(docs.get())
    });

    view! {
        <div class="h-full w-full bg-[#f7f7f7] flex flex-col font-sans select-none relative">
             <crate::components::input_modal::InputModal
                 show=show_create 
                 set_show=set_show_create
                 title=Signal::derive(move || if let Some(p) = create_parent.get() { format!("Create in '{}'", p) } else { "Create New Document".to_string() })
                 initial_value=Signal::derive(move || None::<String>)
                 placeholder="filename or folder/filename"
                 confirm_label="Create"
                 on_confirm=confirm_create
             />
             
             <crate::components::input_modal::InputModal
                 show=show_rename
                 set_show=set_show_rename
                 title=Signal::derive(move || "Rename".to_string())
                 initial_value=Signal::derive(move || Some(rename_target.get().split('/').last().unwrap_or("").to_string()))
                 placeholder="New name"
                 confirm_label="Rename"
                 on_confirm=confirm_rename
             />
             
             <crate::components::input_modal::InputModal
                 show=show_move
                 set_show=set_show_move
                 title=Signal::derive(move || "Move to...".to_string())
                 initial_value=Signal::derive(move || Some(move_source.get())) // Default to current path
                 placeholder="New path (e.g. folder/file.md)"
                 confirm_label="Move"
                 on_confirm=confirm_move
             />
        
            <div class="flex-none h-12 flex items-center justify-between px-3 border-b border-gray-100 hover:bg-gray-100 transition-colors group">
                 // Header content...
                <div class="flex items-center gap-2 overflow-hidden text-gray-700">
                    <div class="p-1 rounded text-gray-400 hover:bg-gray-200 cursor-pointer">
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                          <path fill-rule="evenodd" d="M5.23 7.21a.75.75 0 011.06.02L10 11.168l3.71-3.938a.75.75 0 111.08 1.04l-4.25 4.5a.75.75 0 01-1.08 0l-4.25-4.5a.75.75 0 01.02-1.06z" clip-rule="evenodd" />
                        </svg>
                    </div>
                    <span class="font-medium text-sm truncate">
                        "Knowledge Base"
                    </span>
                </div>
                
                <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                   <button 
                        class="p-1 rounded hover:bg-gray-200 text-gray-500"
                        title="New Doc"
                        on:click=move |_| request_create.run(None)
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
                          <path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" />
                        </svg>
                    </button>
                </div>
            </div>
            
            <div class="flex-1 overflow-y-auto py-2">
                <For
                    each=move || tree_nodes.get()
                    key=|node| node.path.clone()
                    children=move |node| {
                        view! {
                            <div class="relative">
                                <FileTreeItem 
                                    node=node.clone()
                                    current_doc=current_doc 
                                    on_select=on_select 
                                    on_create_click=request_create.clone()
                                    on_menu_click=on_menu_click.clone()
                                    on_menu_close=close_menu.clone()
                                    active_menu=active_menu
                                    on_rename_req=request_rename.clone()
                                    on_delete_req=request_delete.clone()
                                    on_copy_req=on_copy.clone()
                                    on_move_req=request_move.clone()
                                    depth=0 
                                />
                            </div>
                        }
                    }
                />
            </div>

            <div class="flex-none p-3 border-t border-gray-200 bg-gray-50 text-xs text-gray-500 relative">
                 <div class="flex justify-between items-center group">
                     <button 
                        class="flex items-center gap-2 hover:bg-gray-200 px-2 py-1 rounded transition-colors"
                        on:click=move |_| set_show_peers.update(|b| *b = !*b)
                     >
                        <div class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                        <span>
                            {move || {
                                let count = core.peers.get().len();
                                if count == 0 {
                                    "No Peers".to_string()
                                } else {
                                    format!("{} Peer{}", count, if count > 1 { "s" } else { "" })
                                }
                            }}
                        </span>
                     </button>
                     
                     <div class="flex items-center gap-2">
                        <span>{core.status_text}</span>
                     </div>
                 </div>

                 {move || if show_peers.get() {
                    let peers_list = core.peers.get().keys().cloned().collect::<Vec<_>>();
                    view! {
                        <div class="absolute bottom-full left-0 w-64 mb-2 bg-white border border-gray-200 rounded-lg shadow-xl overflow-hidden z-50">
                            <div class="p-2 bg-gray-50 border-b border-gray-100 font-semibold text-gray-700">
                                "Connected Peers"
                            </div>
                            <ul class="max-h-48 overflow-y-auto">
                                {peers_list.into_iter().map(|peer_id| {
                                    let id_str = peer_id.to_string();
                                    let short_id = if id_str.len() > 8 { format!("{}...", &id_str[0..8]) } else { id_str.clone() };
                                    view! {
                                        <li class="p-2 hover:bg-blue-50 cursor-pointer flex justify-between items-center">
                                            <div class="flex items-center gap-2">
                                                <div class="w-1.5 h-1.5 bg-blue-400 rounded-full"></div>
                                                <span class="font-mono text-gray-600" title=id_str.clone()>{short_id}</span>
                                            </div>
                                            <span class="text-[10px] text-gray-400">"Online"</span>
                                        </li>
                                    }
                                }).collect::<Vec<_>>()}
                                {move || if core.peers.get().is_empty() {
                                    view! { <li class="p-4 text-center text-gray-400 italic">"Scanning for peers..."</li> }.into_any()
                                } else {
                                    view! {}.into_any()
                                }}
                            </ul>
                        </div>
                    }.into_any()
                 } else {
                    view! {}.into_any()
                 }}
            </div>
        </div>
    }
}
