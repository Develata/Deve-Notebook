use leptos::prelude::*;
use deve_core::models::DocId;
use crate::components::sidebar_menu::{SidebarMenu, MenuAction};
use super::tree::FileNode;

#[component]
pub fn FileTreeItem(
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
    let is_menu_open = Memo::new(move |_| active_menu.get() == Some(path_check.clone()));
    
    // Build unified action handler
    let rename_req = on_rename_req.clone();
    let delete_req = on_delete_req.clone();
    let path_for_action = node.path.clone();
    let handle_action = Callback::new(move |action: MenuAction| {
        leptos::logging::log!("item.rs handle_action called: action={:?}", action);
        let path = path_for_action.clone();
        match action {
            MenuAction::Rename => rename_req.run(path),
            MenuAction::Delete => delete_req.run(path),
            MenuAction::CopyLink => {
                // Copy path to clipboard using execCommand fallback
                copy_to_clipboard(&path);
                leptos::logging::log!("Copied to clipboard: {}", path);
            }
            MenuAction::OpenInNewWindow => {
                // Open in new browser tab
                if let Some(window) = web_sys::window() {
                    if let Ok(href) = window.location().href() {
                        let url = format!("{}?doc={}", href, path);
                        let _ = window.open_with_url_and_target(&url, "_blank");
                    }
                }
            }
            MenuAction::MoveTo => {
                leptos::logging::log!("Move to... not implemented yet: {}", path);
                // TODO: Show move dialog
            }
            MenuAction::Duplicate => {
                leptos::logging::log!("Duplicate not implemented yet: {}", path);
                // TODO: Implement duplicate
            }
        }
    });
    
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
                            on_action=handle_action 
                            on_close=on_close_clone 
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

/// Copy text to clipboard (stub implementation - logs for now)
/// TODO: Implement via JS interop for full clipboard support
fn copy_to_clipboard(text: &str) {
    // For now, just log. Full clipboard API requires navigator.clipboard
    // which needs specific web-sys features or JS interop.
    leptos::logging::log!("Copy requested: {}", text);
}
