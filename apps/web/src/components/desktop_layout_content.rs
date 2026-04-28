//! plan_ref:
//!   - 08_ui_design_01_web#web-layout-persistence
//!   - 15_release#runtime-observability
//!
use crate::components::dashboard::Dashboard;
use crate::components::diff_view::DiffView;
use crate::editor::Editor;
use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

#[component]
pub fn DesktopLayoutContent(core: CoreState) -> impl IntoView {
    let on_stats = core.on_stats;
    let diff_content = core.diff_content;
    let set_diff_content = core.set_diff_content;
    let current_repo_id = core.current_repo_id;
    let current_repo = core.current_repo;
    let current_scope_nonce = core.current_scope_nonce;
    let is_spectator = core.is_spectator;
    let ws = core.ws.clone();
    let current_editor_doc = Signal::derive({
        let core = core.clone();
        move || {
            if core.pending_branch_switch.get().is_some()
                || core.pending_repo_switch.get().is_some()
            {
                None
            } else {
                core.current_doc.get()
            }
        }
    });

    view! {
        <div class="flex-1 bg-panel shadow-sm border border-default rounded-lg overflow-hidden relative flex flex-col min-w-0">
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
