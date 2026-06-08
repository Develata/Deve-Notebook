//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
//! Browser document client runtime.
//!
//! This adapter owns only client-side document coordination: selected document,
//! pending overlay projection, and document typed intents. Ledger authority
//! remains in server/core.

use crate::runtime::document::pending::PendingLocalEdits;
use deve_core::models::DocId;
use deve_core::tree::FileNode;
use leptos::prelude::*;

#[allow(dead_code)]
#[derive(Clone)]
pub struct DocumentClient {
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub set_current_doc: WriteSignal<Option<DocId>>,
    pub set_explicit_home: WriteSignal<bool>,
    pub pending_local_edits: ReadSignal<PendingLocalEdits>,
    pub set_pending_local_edits: WriteSignal<PendingLocalEdits>,
    pub on_doc_select: Callback<DocId>,
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
    pub tree_nodes: ReadSignal<Vec<FileNode>>,
}
