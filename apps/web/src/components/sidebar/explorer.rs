// apps\web\src\components\sidebar
//! # ExplorerView 组件 (ExplorerView Component)
//!
//! 侧边栏的主要文件浏览器视图。
//! 管理文件树的渲染，以及创建、重命名、移动、删除和上下文菜单的状态。

use crate::components::sidebar::item::FileTreeItem;
use crate::components::sidebar::modals::{ModalState, SidebarModals};
use crate::components::sidebar::tree::{FileNode, build_file_tree};
use crate::components::sidebar::types::FileActionsContext;
use crate::hooks::use_core::CoreState;
use deve_core::models::DocId;
use deve_core::tree::FileNode as CoreFileNode;
use leptos::prelude::*;

use crate::components::main_layout::SearchControl;

#[component]
pub fn ExplorerView(
    docs: ReadSignal<Vec<(DocId, String)>>,
    current_doc: ReadSignal<Option<DocId>>,
    #[prop(into)] on_select: Callback<DocId>,
    #[prop(into)] on_create: Callback<String>,
    #[prop(into)] on_rename: Callback<(String, String)>,
    #[prop(into)] on_delete: Callback<String>,
    #[prop(into)] on_copy: Callback<(String, String)>,
    #[prop(into)] on_move: Callback<(String, String)>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<crate::i18n::Locale>>().expect("locale context");
    let search_control = expect_context::<SearchControl>();
    // 统一模态框状态 (Unified Modal State)
    let (modal_state, set_modal_state) = signal(ModalState::None);

    // 上下文菜单状态
    let (active_menu, set_active_menu) = signal(None::<String>);

    // 剪贴板状态
    let (clipboard_path, set_clipboard_path) = signal(None::<String>);
    provide_context(clipboard_path);
    provide_context(set_clipboard_path);

    // 回调函数
    let request_create = Callback::new(move |parent: Option<String>| {
        let prefix = "+";
        let path = parent.map(|p| format!("{}/", p)).unwrap_or_default();
        search_control.set_mode.set(format!("{}{}", prefix, path));
        search_control.set_show.set(true);
    });

    let confirm_create = Callback::new(move |name: String| {
        if let ModalState::Create { parent } = modal_state.get_untracked() {
            let full_path = if let Some(p) = parent {
                format!("{}/{}", p, name)
            } else {
                name
            };
            on_create.run(full_path);
            set_modal_state.set(ModalState::None);
        }
    });

    let request_rename = Callback::new(move |path: String| {
        set_modal_state.set(ModalState::Rename { target: path });
    });

    let confirm_rename = Callback::new(move |new_name: String| {
        if let ModalState::Rename { target: old } = modal_state.get_untracked() {
            let parent = std::path::Path::new(&old)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("");
            let parent = parent.replace("\\", "/");
            let new_path = if parent.is_empty() {
                new_name
            } else {
                format!("{}/{}", parent, new_name)
            };
            on_rename.run((old, new_path));
            set_modal_state.set(ModalState::None);
        }
    });

    let request_move = Callback::new(move |path: String| {
        set_modal_state.set(ModalState::Move { source: path });
    });

    let confirm_move = Callback::new(move |dest_path: String| {
        if let ModalState::Move { source: src } = modal_state.get_untracked() {
            on_move.run((src, dest_path));
            set_modal_state.set(ModalState::None);
        }
    });

    let request_delete = Callback::new(move |path: String| {
        on_delete.run(path);
    });

    let on_menu_click = Callback::new(move |(path, _ev): (String, web_sys::MouseEvent)| {
        set_active_menu.update(|curr| {
            if *curr == Some(path.clone()) {
                *curr = None;
            } else {
                *curr = Some(path);
            }
        });
    });

    let close_menu = Callback::new(move |_| {
        let set_active = set_active_menu.clone();
        request_animation_frame(move || {
            set_active.set(None);
        });
    });

    // Create Context
    let actions = FileActionsContext {
        current_doc,
        on_select,
        on_create: request_create.clone(),
        on_menu_open: on_menu_click.clone(),
        on_menu_close: close_menu.clone(),
        active_menu,
        on_rename: request_rename.clone(),
        on_delete: request_delete.clone(),
        on_copy: on_copy.clone(),
        on_move: request_move.clone(),
    };
    provide_context(actions);

    // 使用 CoreState 中的 tree_nodes（增量更新），如果为空则回退到 build_file_tree
    let core = expect_context::<CoreState>();
    let tree_nodes = Memo::new(move |_| {
        let core_nodes = core.tree_nodes.get();
        if core_nodes.is_empty() {
            // 回退到传统方式
            build_file_tree(docs.get())
        } else {
            // 转换 CoreFileNode -> FileNode (UI 层类型)
            core_nodes.into_iter().map(convert_node).collect()
        }
    });

    // 转换函数: CoreFileNode -> FileNode
    fn convert_node(node: CoreFileNode) -> FileNode {
        FileNode {
            id: node.doc_id,
            name: node.name,
            path: node.path,
            children: node.children.into_iter().map(convert_node).collect(),
        }
    }

    // Derived active repo label
    let active_repo_label = Signal::derive(move || {
        core.current_repo
            .get()
            .unwrap_or_else(|| "Knowledge Base".to_string())
    });

    view! {
        <div class="h-full w-full bg-[#f7f7f7] flex flex-col font-sans select-none relative">
             <SidebarModals
                 modal_state=modal_state
                 set_modal_state=set_modal_state
                 on_confirm_create=confirm_create
                 on_confirm_rename=confirm_rename
                 on_confirm_move=confirm_move
             />

            <div class="flex-none h-12 flex items-center justify-between px-3 border-b border-gray-100 hover:bg-gray-100 transition-colors group">
                <div class="flex items-center gap-2 flex-1 min-w-0 text-gray-700">
                    <crate::components::sidebar::repo_switcher::RepoSwitcher />
                    <div class="overflow-hidden flex-1">
                        <span class="font-medium text-sm truncate block" title=move || active_repo_label.get()>
                            {move || active_repo_label.get()}
                        </span>
                    </div>
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
                {move || {
                    let nodes = tree_nodes.get();
                    if nodes.is_empty() {
                         view! {
                            <div class="flex flex-col items-center justify-center h-32 text-gray-400 text-sm italic select-none">
                                {move || crate::i18n::t::sidebar::no_docs(locale.get())}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <For
                                each=move || nodes.clone()
                                key=|node| node.path.clone()
                                children=move |node| {
                                    view! {
                                        <div class="relative">
                                            <FileTreeItem
                                                node=node.clone()
                                                depth=0
                                            />
                                        </div>
                                    }
                                }
                            />
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
