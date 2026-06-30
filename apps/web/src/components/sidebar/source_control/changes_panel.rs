//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 04_repository#repo-scope-runtime
//!
use crate::components::sidebar::source_control::changes::Changes;
use crate::hooks::use_core::SourceControlContext;
use leptos::prelude::*;

pub(crate) fn changes_panel_visible(visible: bool, read_blocked: bool) -> bool {
    visible && !read_blocked
}

pub(crate) fn source_control_read_gate_marker() -> &'static str {
    "visible"
}

#[component]
pub fn ChangesPanel(visible: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();

    view! {
        <Show when=move || changes_panel_visible(visible.get(), core.read_block.get().is_some())>
            <div
                class="border-t border-default"
                data-deve-source-control-read-gate=source_control_read_gate_marker()
            >
                <Changes />
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::{changes_panel_visible, source_control_read_gate_marker};

    #[test]
    fn mobile_source_control_read_gate_renders_panel() {
        assert!(changes_panel_visible(true, false));
        assert!(!changes_panel_visible(true, true));
        assert!(!changes_panel_visible(false, false));
    }

    #[test]
    fn mobile_source_control_read_gate_marker_is_stable() {
        assert_eq!(source_control_read_gate_marker(), "visible");
    }
}
