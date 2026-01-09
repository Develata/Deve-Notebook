use leptos::prelude::*;
use deve_core::models::DocId;

// Tree Node Structure
#[derive(Clone, Debug, PartialEq)]
pub struct FileNode {
    pub id: Option<DocId>, // None for folders
    pub name: String,
    pub children: Vec<FileNode>,
    pub path: String, // Full path for creation context
}

// Internal helper for building tree
struct TempNode {
    id: Option<DocId>,
    children: std::collections::BTreeMap<String, TempNode>,
}

fn build_file_tree(docs: Vec<(DocId, String)>) -> Vec<FileNode> {
    let mut root = TempNode { id: None, children: std::collections::BTreeMap::new() };
    
    for (id, path) in docs {
        let parts: Vec<&str> = path.split('/').collect();
        let mut current = &mut root;
        
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            
            // Navigate or Create
            current = current.children.entry(part.to_string()).or_insert_with(|| TempNode {
                id: None,
                children: std::collections::BTreeMap::new(),
            });

            if is_last {
                current.id = Some(id);
            }
        }
    }
    
    // Recursive conversion
    fn convert(name: String, node: TempNode, path_prefix: String) -> FileNode {
        let full_path = if path_prefix.is_empty() { name.clone() } else { format!("{}/{}", path_prefix, name) };
        let children: Vec<FileNode> = node.children.into_iter()
            .map(|(n, c)| convert(n, c, full_path.clone()))
            .collect();
        
        FileNode {
            id: node.id,
            name,
            children,
            path: full_path,
        }
    }
    
    root.children.into_iter()
        .map(|(n, c)| convert(n, c, "".to_string()))
        .collect()
}

use crate::components::sidebar_menu::SidebarMenu; // Import at top level (assuming context)

#[component]
fn FileTreeItem(
    node: FileNode,
    current_doc: ReadSignal<Option<DocId>>,
    on_select: Callback<DocId>,
    on_create_click: Callback<Option<String>>, 
    on_menu_click: Callback<(String, web_sys::MouseEvent)>, 
    on_menu_close: Callback<()>,
    active_menu: ReadSignal<Option<String>>,
    on_rename_req: Callback<String>, 
    on_delete_req: Callback<String>, 
    depth: usize,
) -> impl IntoView {
    let (is_expanded, set_expanded) = signal(true); 
    let is_folder = node.id.is_none();
    
    let padding = format!("padding-left: {}px", depth * 12 + 8);
    
    let on_create_clone = on_create_click.clone();
    let path_create = node.path.clone();
    
    let trigger_create = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        on_create_clone.run(Some(path_create.clone()));
    };

    let on_menu_clone = on_menu_click.clone();
    let path_menu = node.path.clone();
    let trigger_menu = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        on_menu_clone.run((path_menu.clone(), ev));
    };

    let path_check = node.path.clone();
    // Use Memo to allow multiple usages (Memo is Copy)
    let is_menu_open = Memo::new(move |_| active_menu.get() == Some(path_check.clone()));
    
    let rename_req = on_rename_req.clone();
    let path_rename = node.path.clone();
    let handle_rename = Callback::new(move |_| {
        leptos::logging::log!("Sidebar: handle_rename called for {}", path_rename);
        rename_req.run(path_rename.clone())
    });
    
    let delete_req = on_delete_req.clone();
    let path_delete = node.path.clone();
    let handle_delete = Callback::new(move |_| delete_req.run(path_delete.clone()));
    
    let on_close_clone = on_menu_close.clone();

    view! {
        <div class="relative">
            <div 
                class=move || {
                     let base = "relative flex items-center gap-1 py-1 pr-2 cursor-pointer hover:bg-gray-100 text-sm transition-colors select-none group";
                     if let Some(id) = node.id {
                         if current_doc.get() == Some(id) {
                            return format!("{} bg-[#e6f7ff] text-[#008dff]", base);
                         }
                     }
                     format!("{} text-gray-700", base)
                }
                style={padding}
                on:click=move |_| {
                    if is_folder {
                        set_expanded.update(|b| *b = !*b);
                    } else if let Some(id) = node.id {
                        on_select.run(id);
                    }
                }
            >
                // Icon
                <div class=move || if is_folder { "text-gray-400" } else { "text-gray-300" }>
                     {if is_folder {
                         view! {
                             <div class="transition-transform duration-200" style=move || if is_expanded.get() { "transform: rotate(90deg)" } else { "" }>
                                 <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                                   <path fill-rule="evenodd" d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z" clip-rule="evenodd" />
                                 </svg>
                             </div>
                         }.into_any()
                     } else {
                         view! {
                             <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4 opacity-50">
                               <path stroke-linecap="round" stroke-linejoin="round" d="M19.5 14.25v-2.625a3.375 3.375 0 00-3.375-3.375h-1.5A1.125 1.125 0 0113.5 7.125v-1.5a3.375 3.375 0 00-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 00-9-9z" />
                             </svg>
                         }.into_any()
                     }}
                </div>
                
                <span class="truncate flex-1 text-gray-700">{node.name.clone()}</span>
                
                // Actions (Visible on Hover via Opacity)
                <div 
                    class="flex items-center gap-1 pr-1 opacity-0 group-hover:opacity-100 transition-opacity duration-200"
                    class:opacity-100=move || is_menu_open.get() // Keep visible if menu is open
                >
                    // Menu Button
                    <button 
                        class="p-1 rounded hover:bg-gray-300 text-gray-500 transition-colors" 
                        title="More"
                        on:click=trigger_menu
                    >
                         <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                           <path d="M10 3a1.5 1.5 0 110 3 1.5 1.5 0 010-3zM10 8.5a1.5 1.5 0 110 3 1.5 1.5 0 010-3zM11.5 15.5a1.5 1.5 0 10-3 0 1.5 1.5 0 003 0z" />
                         </svg>
                    </button>
                    
                    {if is_folder {
                        view! {
                            // New File Button
                            <button 
                                class="p-1 rounded hover:bg-gray-300 text-gray-600 transition-colors"
                                title="New File"
                                on:click=trigger_create
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                                  <path d="M10.75 4.75a.75.75 0 00-1.5 0v4.5h-4.5a.75.75 0 000 1.5h4.5v4.5a.75.75 0 001.5 0v-4.5h4.5a.75.75 0 000-1.5h-4.5v-4.5z" />
                                </svg>
                            </button>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>
                
                // Context Menu
                {move || if is_menu_open.get() {
                    view! {
                        <SidebarMenu 
                            on_close=on_close_clone 
                            on_rename=handle_rename 
                            on_delete=handle_delete
                        />
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
            
            // Children
            <div class=move || if is_expanded.get() { "block" } else { "hidden" }>
                <For
                    each=move || node.children.clone()
                    key=|child| child.path.clone()
                    children=move |child| {
                        view! {
                            <FileTreeItem 
                                node=child 
                                current_doc=current_doc 
                                on_select=on_select 
                                on_create_click=on_create_click.clone() 
                                on_menu_click=on_menu_click.clone()
                                on_menu_close=on_menu_close.clone()
                                active_menu=active_menu
                                on_rename_req=on_rename_req.clone()
                                on_delete_req=on_delete_req.clone()
                                depth={depth + 1} 
                            />
                        }
                    }
                />
            </div>
        </div>
    }.into_any()
}

#[component]
pub fn Sidebar(
    docs: ReadSignal<Vec<(DocId, String)>>,
    current_doc: ReadSignal<Option<DocId>>,
    #[prop(into)] on_select: Callback<DocId>,
    #[prop(into)] on_create: Callback<String>,
    #[prop(into)] on_rename: Callback<(String, String)>,
    #[prop(into)] on_delete: Callback<String>,
) -> impl IntoView {
    
    // Create Modal State
    let (show_create, set_show_create) = signal(false);
    let (create_parent, set_create_parent) = signal(None::<String>);
    
    // Rename Modal State
    let (show_rename, set_show_rename) = signal(false);
    let (rename_target, set_rename_target) = signal(String::new());
    
    // Context Menu State
    let (active_menu, set_active_menu) = signal(None::<String>);
    
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

    let request_delete = Callback::new(move |path: String| {
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
         // set_timeout(move || set_active_menu.set(None), std::time::Duration::from_millis(0));
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
                        // We need to render the SidebarMenu here if this node is active?
                        // Or pass props down.
                        // Since FileTreeItem is recursive, it's easier to handle local menu rendering if we pass the "active" state down.
                        // And we also need to pass the "close_menu" callback.
                        
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
                                    depth=0 
                                />
                                // Check if this node OR ANY CHILD is active? No, just check path equality inside Item.
                                // But FileTreeItem needs to render the SidebarMenu.
                                // I updated FileTreeItem to accept `active_menu`.
                                // AND we need to pass `close_menu`. 
                                // I missed adding `close_menu` to FileTreeItem props in the signature above.
                                // I will fix that in the real `replace_file_content` if needed, OR just pass a Callback wrapper.
                                
                                // Actually, I forgot to render SidebarMenu *inside* FileTreeItem in the view! above.
                                // I will insert the SidebarMenu component in the `is_menu_open` block.
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}
