use crate::hooks::use_core::state::CoreSignals;
use deve_core::models::DocId;
use leptos::prelude::{GetUntracked, Set};

pub fn reconcile_doc_selection(docs: &[(DocId, String)], signals: CoreSignals) {
    if let Some(selected) = signals.current_doc.get_untracked()
        && !docs.iter().any(|(doc_id, _)| *doc_id == selected)
    {
        leptos::logging::log!("清理过期 current_doc: {} 不在当前 DocList 中", selected);
        signals.set_current_doc.set(None);
    }
    if signals.current_doc.get_untracked().is_none()
        && let Some(pending_path) = signals.pending_created_doc_path.get_untracked()
        && let Some((doc_id, _)) = docs.iter().find(|(_, path)| *path == pending_path)
    {
        signals.set_current_doc.set(Some(*doc_id));
        signals.set_pending_created_doc_path.set(None);
    }
}
