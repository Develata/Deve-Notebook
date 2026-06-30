// apps\web\src\components\sidebar
//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 10_rendering#document-authority-bridge
//!
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

use crate::components::sidebar::types::FileActionsContext;
use crate::components::sidebar_menu::SidebarMenu;
use crate::components::touch_feedback::interactive_item_state_class;
use crate::context_action::ContextActionTarget;
use deve_core::tree::FileNode;
use leptos::prelude::*;

mod action;

use action::{create_action_handler, create_menu_anchor};

#[component]
pub fn FileTreeItem(node: FileNode, #[prop(default = 0)] depth: usize) -> impl IntoView {
    let actions = expect_context::<FileActionsContext>();

    let (is_expanded, set_expanded) = signal(true);
    let is_folder = node.doc_id.is_none();
    let action_target = ContextActionTarget::from_file_tree_node(is_folder, &node.path);

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
        on_menu_clone.run((path_menu.clone(), create_menu_anchor(ev.current_target())));
    };

    let path_check = node.path.clone();
    let active_menu = actions.active_menu;
    let menu_anchor = actions.menu_anchor;
    let is_menu_open = Memo::new(move |_| active_menu.get() == Some(path_check.clone()));

    // 构建统一的操作处理程序
    let delete_req = actions.on_delete.clone();
    let copy_absolute_path = actions.on_copy_absolute_path.clone();
    let reveal_in_system_explorer = actions.on_reveal_in_system_explorer.clone();
    let is_readonly = actions.is_readonly;
    let context_action_readiness = actions.context_action_readiness;
    let open_search = actions.on_open_search.clone();
    let handle_action = create_action_handler(
        context_action_readiness,
        delete_req,
        open_search,
        copy_absolute_path,
        reveal_in_system_explorer,
    );

    let on_close_clone = actions.on_menu_close.clone();
    let current_doc = actions.current_doc;
    let on_select = actions.on_select;

    view! {
        <div class="relative">
            <div
                class=move || {
                     let base = "relative flex items-center gap-1 py-1 pr-2 cursor-pointer text-sm transition-colors select-none group";
                     let selected = node.doc_id.is_some_and(|id| current_doc.get() == Some(id));
                     format!("{} {}", base, interactive_item_state_class(selected, true))
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
                <span class="truncate flex-1">{node.name.clone()}</span>
                // Actions (Visible on Hover via Opacity)
                <crate::components::sidebar::components::ItemActions
                    is_folder=is_folder
                    is_readonly=is_readonly
                    is_menu_open=is_menu_open
                    on_menu=Callback::new(trigger_menu)
                    on_create=Callback::new(trigger_create)
                />
                // Context Menu
                {move || if is_menu_open.get() {
                    view! {
                        <SidebarMenu
                            readiness=context_action_readiness
                            target=action_target.clone()
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
