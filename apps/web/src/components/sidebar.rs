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

#[component]
fn FileTreeItem(
    node: FileNode,
    current_doc: ReadSignal<Option<DocId>>,
    on_select: Callback<DocId>,
    on_create_click: Callback<Option<String>>, // Pass target path (None for root, Some for folder)
    depth: usize,
) -> impl IntoView {
    let (is_expanded, set_expanded) = signal(true); // Default open
    let is_folder = node.id.is_none();
    
    let padding = format!("padding-left: {}px", depth * 12 + 8);
    
    let on_create_clone = on_create_click.clone();
    
    // Action to create file inside this folder
    let trigger_create = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        // Trigger modal with this folder as parent
        on_create_clone.run(Some(node.path.clone()));
    };

    view! {
        <div>
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
                // Use opacity-0 by default, opacity-100 on group-hover. Always flex to maintain layout.
                <div class="flex items-center gap-1 pr-1 opacity-0 group-hover:opacity-100 transition-opacity duration-200">
                    {if is_folder {
                        view! {
                            <>
                                // Menu Button
                                <button class="p-1 rounded hover:bg-gray-300 text-gray-500 transition-colors" title="More">
                                     <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                                       <path d="M10 3a1.5 1.5 0 110 3 1.5 1.5 0 010-3zM10 8.5a1.5 1.5 0 110 3 1.5 1.5 0 010-3zM11.5 15.5a1.5 1.5 0 10-3 0 1.5 1.5 0 003 0z" />
                                     </svg>
                                </button>
                                
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
                            </>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>
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
) -> impl IntoView {
    
    let (show_modal, set_show_modal) = signal(false);
    let (create_parent, set_create_parent) = signal(None::<String>);
    
    // Callback to open modal
    let request_create = Callback::new(move |parent: Option<String>| {
        set_create_parent.set(parent);
        set_show_modal.set(true);
    });
    
    let request_create_clone = request_create.clone();

    let tree_nodes = Memo::new(move |_| {
        build_file_tree(docs.get())
    });

    view! {
        <div class="h-full w-full bg-[#f7f7f7] flex flex-col font-sans select-none relative">
             <crate::components::create_modal::CreateModal 
                 show=show_modal 
                 set_show=set_show_modal
                 parent_path=create_parent
                 on_create=on_create
             />
        
            <div class="flex-none h-12 flex items-center justify-between px-3 border-b border-gray-100 hover:bg-gray-100 transition-colors group">
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
                            <FileTreeItem 
                                node=node 
                                current_doc=current_doc 
                                on_select=on_select 
                                on_create_click=request_create_clone.clone()
                                depth=0 
                            />
                        }
                    }
                />
            </div>
        </div>
    }
}
