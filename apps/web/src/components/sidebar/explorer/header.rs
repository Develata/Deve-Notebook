//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::icons::Plus;
use crate::components::main_layout::SearchControl;
use crate::hooks::use_core::write_gate::{RepoWriteSignals, repo_write_block_tracked};
use crate::hooks::use_core::{BranchContext, EditorContext};
use crate::i18n::{Locale, t};
use crate::runtime::{
    document_client::DocumentClient, rendering_client::RenderingClient, scope_client::ScopeClient,
    session_client::SessionClient,
};
use leptos::prelude::*;

#[component]
pub(super) fn ExplorerHeader(
    locale: RwSignal<Locale>,
    search_control: SearchControl,
    is_readonly: Signal<bool>,
) -> impl IntoView {
    let branch = expect_context::<BranchContext>();
    let document = expect_context::<DocumentClient>();
    let session = expect_context::<SessionClient>();
    let rendering = expect_context::<RenderingClient>();
    let scope = expect_context::<ScopeClient>();
    let editor = expect_context::<EditorContext>();
    let docs_for_create = document.docs;
    let request_create = Callback::new(move |parent: Option<String>| {
        search_control.set_mode.set(super::new_doc_search_query(
            docs_for_create,
            parent.as_deref(),
        ));
        search_control.set_show.set(true);
    });

    let branch_for_title = branch.clone();
    let branch_for_label = branch.clone();
    let session_for_write = session.clone();
    let rendering_for_write = rendering.clone();
    let scope_for_write = scope.clone();
    let editor_for_write = editor.clone();

    view! {
        <div class="flex-none h-12 flex items-center justify-between px-3 border-b border-default hover:bg-hover transition-colors group">
            <div class="flex items-center gap-2 flex-1 min-w-0 text-primary">
                <crate::components::sidebar::repo_switcher::RepoSwitcher />
                <div class="overflow-hidden flex-1">
                    <span class="font-medium text-sm truncate block" title=move || explorer_active_repo_label(&branch_for_title, locale)>
                        {move || explorer_active_repo_label(&branch_for_label, locale)}
                    </span>
                </div>
            </div>

            <Show when=move || {
                explorer_can_write(
                    &session_for_write,
                    &rendering_for_write,
                    &scope_for_write,
                    &editor_for_write,
                ) && !is_readonly.get()
            }>
                <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                        type="button"
                        class="p-1 rounded hover:bg-hover text-secondary"
                        title=move || t::sidebar::new_doc(locale.get())
                        data-deve-new-doc-button="true"
                        on:click=move |_| request_create.run(None)
                    >
                        <Plus />
                    </button>
                </div>
            </Show>
        </div>
    }
}

fn explorer_active_repo_label(branch: &BranchContext, locale: RwSignal<Locale>) -> String {
    branch
        .current_repo
        .get()
        .unwrap_or_else(|| t::sidebar::knowledge_base(locale.get()).to_string())
}

fn explorer_can_write(
    session: &SessionClient,
    rendering: &RenderingClient,
    scope: &ScopeClient,
    editor: &EditorContext,
) -> bool {
    repo_write_block_tracked(
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
    .is_none()
}
