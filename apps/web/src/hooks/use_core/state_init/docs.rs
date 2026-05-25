//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!
use crate::editor::EditorStats;
use deve_core::models::DocId;
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub(super) struct DocSignals {
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub set_docs: WriteSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub set_current_doc: WriteSignal<Option<DocId>>,
    pub pending_created_doc_path: ReadSignal<Option<String>>,
    pub set_pending_created_doc_path: WriteSignal<Option<String>>,
    pub stats: ReadSignal<EditorStats>,
    pub set_stats: WriteSignal<EditorStats>,
    pub doc_version: ReadSignal<u64>,
    pub set_doc_version: WriteSignal<u64>,
    pub playback_version: ReadSignal<u64>,
    pub set_playback_version: WriteSignal<u64>,
}

pub(super) fn init_doc_signals() -> DocSignals {
    let (docs, set_docs) = signal(Vec::<(DocId, String)>::new());
    let (current_doc, set_current_doc) = signal(None::<DocId>);
    let (pending_created_doc_path, set_pending_created_doc_path) = signal(None::<String>);
    let (stats, set_stats) = signal(EditorStats::default());
    let (doc_version, set_doc_version) = signal(0u64);
    let (playback_version, set_playback_version) = signal(0u64);

    DocSignals {
        docs,
        set_docs,
        current_doc,
        set_current_doc,
        pending_created_doc_path,
        set_pending_created_doc_path,
        stats,
        set_stats,
        doc_version,
        set_doc_version,
        playback_version,
        set_playback_version,
    }
}
