//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 11_ui_design/03_mobile#mobile-current-native-boundary
//!
use crate::components::layout_context::EditorContentContext;
use crate::hooks::use_core::CoreState;
use leptos::prelude::*;

pub(super) fn build_mobile_title(core: CoreState) -> Memo<String> {
    Memo::new(move |_| {
        let current = core.current_doc.get();
        if let Some(id) = current {
            let docs = core.docs.get();
            if let Some((_, path)) = docs.iter().find(|(doc_id, _)| *doc_id == id) {
                return path.clone();
            }
        }
        "Deve-Note".to_string()
    })
}

pub(super) fn resolve_content_signal() -> (Option<ReadSignal<String>>, WriteSignal<String>) {
    let content_ctx = use_context::<EditorContentContext>();
    let (outline_content, set_outline_content) = signal(String::new());
    let content_signal = match content_ctx {
        Some(ctx) => Some(ctx.content),
        None => Some(outline_content),
    };
    (content_signal, set_outline_content)
}

pub(super) fn build_doc_select_callback(
    on_select: Callback<deve_core::models::DocId>,
    close_drawers: Callback<()>,
) -> Callback<deve_core::models::DocId> {
    Callback::new(move |id| {
        on_select.run(id);
        close_drawers.run(());
    })
}
