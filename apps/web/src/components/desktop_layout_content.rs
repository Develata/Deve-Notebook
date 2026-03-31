use crate::components::dashboard::Dashboard;
use crate::components::diff_view::DiffView;
use crate::editor::Editor;
use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

#[component]
pub fn DesktopLayoutContent(core: CoreState) -> impl IntoView {
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
    let diff_core = core.clone();

    view! {
        <div class="flex-1 bg-panel shadow-sm border border-default rounded-lg overflow-hidden relative flex flex-col min-w-0">
            <div class="flex-1 min-h-0 overflow-hidden">
                <Show
                    when=move || diff_core.diff_content.get().is_some()
                    fallback=move || view! {
                        <Show
                            when=move || current_editor_doc.get().is_some()
                            fallback=move || view! { <Dashboard /> }
                        >
                            {move || {
                                current_editor_doc.get().map(|doc_id| {
                                    view! { <Editor doc_id=doc_id on_stats=core.on_stats /> }
                                })
                            }}
                        </Show>
                    }
                >
                    {move || {
                        diff_core.diff_content.get().map(|session| {
                            view! {
                                <DiffView
                                    repo_scope=diff_core
                                        .current_repo_id
                                        .get()
                                        .or_else(|| diff_core.current_repo.get())
                                        .unwrap_or_default()
                                    path=session.path
                                    old_content=session.old_content
                                    new_content=session.new_content
                                    is_readonly=diff_core.is_spectator.get()
                                    on_close=Callback::new(move |_| diff_core.set_diff_content.set(None))
                                />
                            }
                        })
                    }}
                </Show>
            </div>
        </div>
    }
}
