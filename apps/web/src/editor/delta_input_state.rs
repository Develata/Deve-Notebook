//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use super::EditorStats;
use super::ffi::getEditorContent;
use leptos::prelude::{Callable, Callback, Set, WriteSignal};

pub(super) fn sync_local_state(
    on_stats: Option<Callback<EditorStats>>,
    set_content: WriteSignal<String>,
) {
    let text = getEditorContent();
    emit_stats(on_stats, &text);
    set_content.set(text);
}

fn emit_stats(on_stats: Option<Callback<EditorStats>>, text: &str) {
    if let Some(cb) = on_stats {
        cb.run(EditorStats {
            chars: text.len(),
            words: text.split_whitespace().count(),
            lines: text.lines().count(),
        });
    }
}
