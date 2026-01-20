// apps\web\src\components\sidebar
//! # FileTreeItem 组件 (文件树节点组件)
//!
//! **架构作用**:
//! 渲染文件树中的单个节点（文件或文件夹），并处理局部交互。
//!
//! **核心功能清单**:
//! - 递归渲染：遇到文件夹时递归渲染子节点。
//! - 交互：点击打开文档，点击展开/折叠文件夹。
//! - 上下文菜单：右键或点击更多按钮触发 `handle_action` (Rename, Copy, Paste, Move, Delete)。
//!
//! **类型**: Core MUST (核心必选)

use leptos::prelude::*;
use deve_core::models::DocId;
use crate::components::sidebar_menu::{SidebarMenu, MenuAction};
use crate::components::sidebar::types::FileActionsContext;
use super::tree::FileNode;

#[component]
pub fn FileTreeItem(
    node: FileNode,
    #[prop(default = 0)]
    depth: usize,
) -> impl IntoView {
    let actions = expect_context::<FileActionsContext>();

    let (is_expanded, set_expanded) = signal(true); 
    let is_folder = node.id.is_none();
    
    let padding = format!("padding-left: {}px", depth * 12 + 8);
    
    let on_create_clone = actions.on_create.clone();
    let path_create = node.path.clone();
    
    let trigger_create = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        on_create_clone.run(Some(path_create.clone()));
    };

    let on_menu_clone = actions.on_menu_open.clone();
    let path_menu = node.path.clone();
    let trigger_menu = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        on_menu_clone.run((path_menu.clone(), ev));
    };

    let path_check = node.path.clone();
    let active_menu = actions.active_menu;
    let is_menu_open = Memo::new(move |_| active_menu.get() == Some(path_check.clone()));
    
    // 剪贴板上下文
    let set_clipboard = use_context::<WriteSignal<Option<String>>>()
        .expect("clipboard set context");
    let clipboard = use_context::<ReadSignal<Option<String>>>()
        .expect("clipboard read context");
    
    // 构建统一的操作处理程序
    let rename_req = actions.on_rename.clone();
    let delete_req = actions.on_delete.clone();
    let copy_req = actions.on_copy.clone();
    let move_req = actions.on_move.clone();
    let path_for_action = node.path.clone();
    let handle_action = Callback::new(move |action: MenuAction| {
        leptos::logging::log!("item.rs handle_action called: action={:?}", action);
        let path = path_for_action.clone();
        match action {
            MenuAction::Rename => rename_req.run(path),
            MenuAction::Delete => delete_req.run(path),
            MenuAction::Copy => {
                // 存储到剪贴板上下文
                set_clipboard.set(Some(path.clone()));
                leptos::logging::log!("Copied to clipboard: {}", path);
            }
            MenuAction::Paste => {
                // Get from clipboard and log (actual paste logic requires backend support)
                if let Some(src) = clipboard.get_untracked() {
                    leptos::logging::log!("Paste requested: copy {} to {}", src, path);
                    
                    // 确定目标文件夹
                    let dest_folder = if is_folder {
                        path.clone()
                    } else {
                        // 当前项目的父级
                         let p = std::path::Path::new(&path).parent().and_then(|p| p.to_str()).unwrap_or("");
                         p.replace('\\', "/")
                    };
                    
                    // 从源确定新文件名
                    let src_name = std::path::Path::new(&src).file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
                    
                    let dest_path = if dest_folder.is_empty() {
                        src_name.to_string()
                    } else {
                        format!("{}/{}", dest_folder, src_name)
                    };
                    
                    if src == dest_path {
                         leptos::logging::warn!("Cannot paste into same location without rename logic");
                         // TODO: Auto-rename (e.g. "copy")
                    } else {
                        copy_req.run((src, dest_path));
                    }
                } else {
                    leptos::logging::log!("Paste: clipboard is empty");
                }
            }
            MenuAction::OpenInNewWindow => {
                // 在新浏览器标签页中打开
                if let Some(window) = web_sys::window() {
                    if let Ok(href) = window.location().href() {
                        let url = format!("{}?doc={}", href, path);
                        let _ = window.open_with_url_and_target(&url, "_blank");
                    }
                }
            }
            MenuAction::MoveTo => {
                 move_req.run(path);
            }
        }
    });
    
    let on_close_clone = actions.on_menu_close.clone();
    let current_doc = actions.current_doc;
    let on_select = actions.on_select;

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
                <crate::components::sidebar::components::FileIcon 
                    is_folder=is_folder
                    is_expanded=is_expanded
                />
                
                <span class="truncate flex-1 text-gray-700">{node.name.clone()}</span>
                
                // Actions (Visible on Hover via Opacity)
                <crate::components::sidebar::components::ItemActions
                    is_folder=is_folder
                    is_menu_open=is_menu_open
                    on_menu=Callback::new(trigger_menu)
                    on_create=Callback::new(trigger_create)
                />
                
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
                                depth={depth + 1} 
                            />
                        }
                    }
                />
            </div>
        </div>
    }.into_any()
}


