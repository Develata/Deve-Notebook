// apps\web\src\components\sidebar
//! # FileTreeItem 组件 (文件树节点组件)
//!
//! **架构作用**:
//! 渲染文件树中的单个节点（文件或文件夹），并处理局部交互。
//!
//! **核心功能清单**:
//! - 递归渲染：遇到文件夹时递归渲染子节点。
//! - 交互：点击打开文档，点击展开/折叠文件夹。
//! - 上下文菜单：右键或点击更多按钮触发 `handle_action` (Rename, Copy, Move, Delete)。
//!
//! **类型**: Core MUST (核心必选)

use crate::components::dropdown::AnchorRect;
use crate::components::sidebar::types::FileActionsContext;
use crate::components::sidebar_menu::{MenuAction, SidebarMenu};
use deve_core::tree::FileNode;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn FileTreeItem(node: FileNode, #[prop(default = 0)] depth: usize) -> impl IntoView {
    let actions = expect_context::<FileActionsContext>();

    let (is_expanded, set_expanded) = signal(true);
    let is_folder = node.doc_id.is_none();

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
        if let Some(target) = ev.current_target()
            && let Ok(el) = target.dyn_into::<web_sys::Element>()
        {
            let rect = el.get_bounding_client_rect();
            let anchor = AnchorRect {
                top: rect.top(),
                bottom: rect.bottom(),
                left: rect.left(),
                right: rect.right(),
            };
            on_menu_clone.run((path_menu.clone(), anchor));
            return;
        }
        let anchor = AnchorRect {
            top: 0.0,
            bottom: 0.0,
            left: 0.0,
            right: 0.0,
        };
        on_menu_clone.run((path_menu.clone(), anchor));
    };

    let path_check = node.path.clone();
    let active_menu = actions.active_menu;
    let menu_anchor = actions.menu_anchor;
    let is_menu_open = Memo::new(move |_| active_menu.get() == Some(path_check.clone()));

    // 构建统一的操作处理程序
    let delete_req = actions.on_delete.clone();
    let open_search = actions.on_open_search.clone();
    let path_for_action = node.path.clone();
    let handle_action = Callback::new(move |action: MenuAction| {
        leptos::logging::log!("item.rs handle_action called: action={:?}", action);
        let path = path_for_action.clone();
        match action {
            MenuAction::Rename => {
                open_search.run(build_prefill_command("mv", &path, None));
            }
            MenuAction::Delete => delete_req.run(path),
            MenuAction::Copy => {
                open_search.run(build_prefill_command("cp", &path, None));
            }
            MenuAction::OpenInNewWindow => {
                // 在新浏览器标签页中打开
                if let Some(window) = web_sys::window()
                    && let Ok(href) = window.location().href()
                {
                    let url = format!("{}?doc={}", href, path);
                    let _ = window.open_with_url_and_target(&url, "_blank");
                }
            }
            MenuAction::MoveTo => {
                open_search.run(build_prefill_command("mv", &path, None));
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
                     let base = "relative flex items-center gap-1 py-1 pr-2 cursor-pointer hover:bg-gray-50 active:bg-gray-100 text-sm transition-colors select-none group";
                     if let Some(id) = node.doc_id
                          && current_doc.get() == Some(id)
                      {
                        return format!("{} bg-blue-50 text-blue-700", base);
                      }
                     format!("{} text-gray-700", base)
                }
                style={padding}
                on:click=move |_| {
                    if is_folder {
                        set_expanded.update(|b| *b = !*b);
                    } else if let Some(id) = node.doc_id {
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
                            anchor=menu_anchor
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

fn build_prefill_command(cmd: &str, src: &str, dst_with_cursor: Option<String>) -> String {
    let src_text = quote_arg(src);
    let dst_text = match dst_with_cursor {
        Some(dst) => format!("\"{}\"", sanitize_arg(&dst)),
        None => "\"|\"".to_string(),
    };
    format!(">{} {} {}", cmd, src_text, dst_text)
}

fn quote_arg(arg: &str) -> String {
    format!("\"{}\"", sanitize_arg(arg))
}

fn sanitize_arg(arg: &str) -> String {
    arg.replace('"', "'")
}
