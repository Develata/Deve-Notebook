//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::components::sidebar::source_control::changes::Changes;
use crate::hooks::use_core::SourceControlContext;
use leptos::prelude::*;

#[component]
pub fn ChangesPanel(visible: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();

    view! {
        <Show when=move || visible.get() && core.active_branch.get().is_none()>
            <div class="border-t border-default">
                <Changes />
            </div>
        </Show>
    }
}
