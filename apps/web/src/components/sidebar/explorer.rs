// apps\web\src\components\sidebar
//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! # ExplorerView 组件 (ExplorerView Component)
//!
//! 侧边栏的主要文件浏览器视图。
//! 管理文件树、顶部动作和上下文菜单状态。

use crate::api::{copy_host_file_absolute_path_to_clipboard, reveal_host_file_in_system_explorer};
use crate::components::sidebar::types::FileActionsContext;
use crate::components::ui_back::{UiBackCoordinator, UiBackLayer};
use crate::context_action::{ContextActionReadiness, ContextActionScope};
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_tracked};
use crate::hooks::use_core::{DocContext, EditorContext};
use crate::i18n::Locale;
use crate::runtime::{
    document_client::DocumentClient, rendering_client::RenderingClient, scope_client::ScopeClient,
    session_client::SessionClient,
};
use deve_core::models::DocId;
use leptos::prelude::*;
use leptos::task::spawn_local;

mod header;
mod tree_view;

use crate::components::dropdown::AnchorRect;
use header::ExplorerHeader;
use tree_view::ExplorerTree;

pub(super) fn new_doc_search_query(
    docs: ReadSignal<Vec<(DocId, String)>>,
    parent: Option<&str>,
) -> String {
    let path = crate::hooks::use_core::doc_name::next_untitled_doc_path(
        docs.get_untracked().iter().map(|(_, path)| path.as_str()),
        parent,
    );
    format!("+{path}")
}

fn context_action_readiness_for_runtime(
    session: &SessionClient,
    rendering: &RenderingClient,
    scope: &ScopeClient,
    editor: &EditorContext,
    readonly: bool,
) -> ContextActionReadiness {
    let action_scope =
        ContextActionScope::new(scope.current_repo_id.get(), scope.current_scope_nonce.get());
    let write_blocked = repo_write_block_tracked(
        &session.ws,
        RepoWriteSignals {
            load_state: rendering.load_state,
            is_spectator: scope.is_spectator,
            handshake_ready: session.handshake_ready,
            current_repo_id: scope.current_repo_id,
            current_scope_nonce: scope.current_scope_nonce,
            active_branch: scope.active_branch,
            pending_branch_switch: editor.pending_branch_switch,
            pending_repo_switch: scope.pending_repo_switch,
        },
    )
    .is_some();

    ContextActionReadiness::new(action_scope, readonly, write_blocked).with_host_file_actions(
        session.ws.host_file_copy_absolute_path.get(),
        session.ws.host_file_reveal_in_system_explorer.get(),
    )
}

#[component]
pub fn ExplorerView(
    _docs: ReadSignal<Vec<(DocId, String)>>,
    current_doc: ReadSignal<Option<DocId>>,
    is_readonly: Signal<bool>,
    #[prop(into)] on_select: Callback<DocId>,
    #[prop(into)] on_delete: Callback<String>,
    #[prop(into)] on_search_open: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let doc = expect_context::<DocContext>();
    let document = expect_context::<DocumentClient>();
    let session = expect_context::<SessionClient>();
    let rendering = expect_context::<RenderingClient>();
    let scope = expect_context::<ScopeClient>();
    let editor = expect_context::<EditorContext>();
    let session_for_context_action = session.clone();
    let rendering_for_context_action = rendering.clone();
    let scope_for_context_action = scope.clone();
    let editor_for_context_action = editor.clone();
    let context_action_readiness = Signal::derive(move || {
        context_action_readiness_for_runtime(
            &session_for_context_action,
            &rendering_for_context_action,
            &scope_for_context_action,
            &editor_for_context_action,
            is_readonly.get(),
        )
    });
    // 上下文菜单状态
    let (active_menu, set_active_menu) = signal(None::<String>);
    let (menu_anchor, set_menu_anchor) = signal(None::<AnchorRect>);
    let ui_back = expect_context::<UiBackCoordinator>();
    ui_back.register(UiBackLayer::Overlay, move || {
        if active_menu.try_get_untracked().flatten().is_some() {
            set_active_menu.set(None);
            set_menu_anchor.set(None);
            return true;
        }
        false
    });

    // 回调函数
    let search_control = expect_context::<crate::components::main_layout::SearchControl>();
    let open_search = Callback::new(move |query: String| {
        super::open_search_overlay(search_control, on_search_open, query);
    });

    let docs_for_create = document.docs;
    let request_create = Callback::new(move |parent: Option<String>| {
        open_search.run(new_doc_search_query(docs_for_create, parent.as_deref()));
    });

    let request_delete = Callback::new(move |path: String| {
        on_delete.run(path);
    });
    let repo_id_for_copy_absolute_path = scope.current_repo_id;
    let copy_absolute_path = Callback::new(move |path: String| {
        let repo_id = repo_id_for_copy_absolute_path.get_untracked();
        spawn_local(async move {
            match copy_host_file_absolute_path_to_clipboard(repo_id, path).await {
                Ok(_) => {
                    leptos::logging::log!("Host file absolute path copied");
                }
                Err(error) => {
                    leptos::logging::error!("Host file absolute path copy failed: {:?}", error);
                }
            }
        });
    });
    let repo_id_for_reveal = scope.current_repo_id;
    let reveal_in_system_explorer = Callback::new(move |path: String| {
        let repo_id = repo_id_for_reveal.get_untracked();
        spawn_local(async move {
            if let Err(error) = reveal_host_file_in_system_explorer(repo_id, path).await {
                leptos::logging::error!("Host file reveal failed: {:?}", error);
            }
        });
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
        context_action_readiness,
        on_select,
        on_create: request_create.clone(),
        on_open_search: open_search.clone(),
        on_menu_open: on_menu_click.clone(),
        on_menu_close: close_menu.clone(),
        active_menu,
        menu_anchor,
        on_delete: request_delete.clone(),
        on_copy_absolute_path: copy_absolute_path,
        on_reveal_in_system_explorer: reveal_in_system_explorer,
    };
    provide_context(actions);

    view! {
        <div class="h-full w-full bg-sidebar flex flex-col font-sans select-none relative">
            <ExplorerHeader
                locale
                search_control
                is_readonly
                on_search_open
            />
            <ExplorerTree locale doc />
        </div>
    }
}
