//! plan_ref:
//!   - 11_ui_design/01_web#web-layout-persistence
//!   - 11_ui_design/index#editor-group-tabstrip
//!   - 18_release#runtime-observability
//!
use super::editor_tabs::EditorTabStrip;
use crate::components::dashboard::Dashboard;
use crate::components::diff_view::DiffView;
use crate::components::editor_tabs::{
    EditorTabRuntimeInputs, create_current_editor_doc, create_editor_tab_runtime,
};
use crate::editor::Editor;
use crate::hooks::use_core::EditorContext;
use crate::runtime::{
    document_client::DocumentClient, rendering_client::RenderingClient, scope_client::ScopeClient,
    session_client::SessionClient, source_control_client::SourceControlClient,
};
use leptos::prelude::*;

#[component]
pub fn DesktopLayoutContent(center_width: Signal<i32>) -> impl IntoView {
    let rendering = expect_context::<RenderingClient>();
    let source_control = expect_context::<SourceControlClient>();
    let scope = expect_context::<ScopeClient>();
    let session = expect_context::<SessionClient>();
    let document = expect_context::<DocumentClient>();
    let editor = expect_context::<EditorContext>();
    let on_stats = rendering.on_stats;
    let diff_content = source_control.diff_content;
    let set_diff_content = source_control.set_diff_content;
    let current_repo_id = scope.current_repo_id;
    let current_repo = scope.current_repo;
    let current_scope_nonce = scope.current_scope_nonce;
    let is_spectator = scope.is_spectator;
    let ws = session.ws.clone();
    let current_editor_doc = create_current_editor_doc(&document, &editor);
    let tabs = create_editor_tab_runtime(
        EditorTabRuntimeInputs {
            document,
            editor,
            scope,
            source_control,
        },
        current_editor_doc,
    );

    view! {
        <div
            data-deve-desktop-col="3-display-editor"
            data-deve-desktop-col-width=move || center_width.get().to_string()
            aria-hidden=move || (center_width.get() == 0).to_string()
            class="bg-panel shadow-sm border border-default rounded-lg overflow-hidden relative flex flex-col min-w-0"
            style=move || {
                if center_width.get() == 0 {
                    "visibility: hidden; pointer-events: none;".to_string()
                } else {
                    String::new()
                }
            }
        >
            <EditorTabStrip
                doc_tabs=tabs.doc_tabs
                diff_tabs=tabs.diff_tabs
                active_tab=tabs.active_tab
                on_select_document=tabs.on_select_document
                on_select_diff=tabs.on_select_diff
                on_close_document=tabs.on_close_document
                on_close_diff=tabs.on_close_diff
            />
            <div class="flex-1 min-h-0 overflow-hidden">
                {move || {
                    if let Some(session) = diff_content.get() {
                            let merge_conflict = session.merge_conflict.clone();
                            let repo_scope = current_repo_id
                                .get()
                                .or_else(|| current_repo.get())
                                .unwrap_or_default();
                            let resolve_ws = ws.clone();
                            let on_resolve = merge_conflict.clone().map(|conflict| {
                                let resolve_ws = resolve_ws.clone();
                                Callback::new(move |(action, result_content)| {
                                    resolve_ws.send(conflict.resolve_message(
                                        action,
                                        result_content,
                                        current_scope_nonce.get_untracked(),
                                    ));
                                    set_diff_content.set(None);
                                })
                            });
                            view! {
                                <DiffView
                                    repo_scope=repo_scope
                                    path=session.path
                                    display_path=session.display_path
                                    old_content=session.old_content
                                    new_content=session.new_content
                                    is_readonly=is_spectator.get()
                                    merge_conflict=merge_conflict
                                    on_resolve_merge_conflict=on_resolve
                                    on_close=Callback::new(move |_| set_diff_content.set(None))
                                />
                            }
                            .into_any()
                    } else if let Some(doc_id) = current_editor_doc.get() {
                        view! { <Editor doc_id=doc_id on_stats=on_stats /> }.into_any()
                    } else {
                        view! { <Dashboard /> }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
