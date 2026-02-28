// apps\web\src\components\sidebar
//! # ExplorerView 组件 (ExplorerView Component)
//!
//! 侧边栏的主要文件浏览器视图。
//! 管理文件树的渲染，以及创建、重命名、移动、删除和上下文菜单的状态。

use crate::components::sidebar::item::FileTreeItem;
use crate::components::sidebar::types::FileActionsContext;
use crate::hooks::use_core::CoreState;
use crate::i18n::t;
use deve_core::models::DocId;
use leptos::prelude::*;

use crate::components::dropdown::AnchorRect;
use crate::components::main_layout::SearchControl;

#[component]
pub fn ExplorerView(
    _docs: ReadSignal<Vec<(DocId, String)>>,
    current_doc: ReadSignal<Option<DocId>>,
    #[prop(into)] on_select: Callback<DocId>,
    #[prop(into)] on_delete: Callback<String>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<crate::i18n::Locale>>().expect("locale context");
    let search_control = expect_context::<SearchControl>();
    // 上下文菜单状态
    let (active_menu, set_active_menu) = signal(None::<String>);
    let (menu_anchor, set_menu_anchor) = signal(None::<AnchorRect>);

    // 回调函数
    let open_search = Callback::new(move |query: String| {
        search_control.set_mode.set(query);
        search_control.set_show.set(true);
    });

    let request_create = Callback::new(move |parent: Option<String>| {
        let prefix = "+";
        let path = parent.map(|p| format!("{}/", p)).unwrap_or_default();
        open_search.run(format!("{}{}", prefix, path));
    });

    let request_delete = Callback::new(move |path: String| {
        on_delete.run(path);
    });

    let on_menu_click = Callback::new(move |(path, anchor): (String, AnchorRect)| {
        set_active_menu.update(|curr| {
            if *curr == Some(path.clone()) {
                *curr = None;
                set_menu_anchor.set(None);
            } else {
                *curr = Some(path);
                set_menu_anchor.set(Some(anchor));
            }
        });
    });

    let close_menu = Callback::new(move |_| {
        let set_active = set_active_menu.clone();
        let set_anchor = set_menu_anchor.clone();
        request_animation_frame(move || {
            set_active.set(None);
            set_anchor.set(None);
        });
    });

    // Create Context
    let actions = FileActionsContext {
        current_doc,
        on_select,
        on_create: request_create.clone(),
        on_open_search: open_search.clone(),
        on_menu_open: on_menu_click.clone(),
        on_menu_close: close_menu.clone(),
        active_menu,
        menu_anchor,
        on_delete: request_delete.clone(),
    };
    provide_context(actions);

    // 使用 TreeDelta 增量更新的树
    let core = expect_context::<CoreState>();
    let tree_nodes = Memo::new(move |_| core.tree_nodes.get());

    // Derived active repo label
    let active_repo_label = Signal::derive(move || {
        core.current_repo
            .get()
            .unwrap_or_else(|| t::sidebar::knowledge_base(locale.get()).to_string())
    });

    view! {
        <div class="h-full w-full bg-[#f7f7f7] flex flex-col font-sans select-none relative">
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
