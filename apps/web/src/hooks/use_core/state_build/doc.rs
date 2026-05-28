//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 04_repository#tree-projection-contract
//!
use deve_core::models::DocId;
use deve_core::tree::FileNode;
use leptos::prelude::*;

use super::super::callbacks::DocCallbacks;
use super::super::navigation::PendingNavigation;
use crate::runtime::document::pending::PendingLocalEdits;
use super::super::state::CoreSignals;

pub(super) struct DocStateSection {
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub set_current_doc: WriteSignal<Option<DocId>>,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
    pub pending_navigation: ReadSignal<Option<PendingNavigation>>,
    pub set_pending_navigation: WriteSignal<Option<PendingNavigation>>,
    pub on_doc_select: Callback<DocId>,
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
    pub tree_nodes: ReadSignal<Vec<FileNode>>,
    pub set_explicit_home: WriteSignal<bool>,
}

pub(super) fn build_doc_section(signals: &CoreSignals, doc: &DocCallbacks) -> DocStateSection {
    DocStateSection {
        docs: signals.docs,
        current_doc: signals.current_doc,
        set_current_doc: signals.set_current_doc,
        pending_local_edits: signals.pending_local_edits,
        set_pending_local_edits: signals.set_pending_local_edits,
        pending_navigation: signals.pending_navigation,
        set_pending_navigation: signals.set_pending_navigation,
        on_doc_select: doc.on_doc_select,
        on_doc_create: doc.on_doc_create,
        on_doc_rename: doc.on_doc_rename,
        on_doc_delete: doc.on_doc_delete,
        on_doc_copy: doc.on_doc_copy,
        on_doc_move: doc.on_doc_move,
        tree_nodes: signals.tree_nodes,
        set_explicit_home: signals.set_explicit_home,
    }
}
