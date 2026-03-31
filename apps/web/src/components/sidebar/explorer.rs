// apps\web\src\components\sidebar
//! # ExplorerView 组件 (ExplorerView Component)
//!
//! 侧边栏的主要文件浏览器视图。
//! 管理文件树、顶部动作和上下文菜单状态。

use crate::components::sidebar::types::FileActionsContext;
use crate::hooks::use_core::DocContext;
use crate::i18n::Locale;
use deve_core::models::DocId;
use leptos::prelude::*;

use crate::components::dropdown::AnchorRect;

#[path = "explorer_header.rs"]
mod header;
#[path = "explorer_tree.rs"]
mod tree_view;

use header::ExplorerHeader;
use tree_view::ExplorerTree;

#[component]
pub fn ExplorerView(
    _docs: ReadSignal<Vec<(DocId, String)>>,
    current_doc: ReadSignal<Option<DocId>>,
    is_readonly: Signal<bool>,
    #[prop(into)] on_select: Callback<DocId>,
    #[prop(into)] on_delete: Callback<String>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    // 上下文菜单状态
    let (active_menu, set_active_menu) = signal(None::<String>);
    let (menu_anchor, set_menu_anchor) = signal(None::<AnchorRect>);

    // 回调函数
    let search_control = expect_context::<crate::components::main_layout::SearchControl>();
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
        is_readonly,
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
    let doc = expect_context::<DocContext>();
    let branch = expect_context::<crate::hooks::use_core::BranchContext>();
    let core = expect_context::<crate::hooks::use_core::CoreState>();

    view! {
        <div class="h-full w-full bg-sidebar flex flex-col font-sans select-none relative">
            <ExplorerHeader
                locale
                branch
                core
                search_control
                is_readonly
            />
            <ExplorerTree locale doc />
        </div>
    }
}
