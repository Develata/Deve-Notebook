use deve_core::models::DocId;
use deve_core::tree::FileNode;
use leptos::prelude::*;

use super::super::types::SearchHit;

#[derive(Clone)]
pub struct DocContext {
    pub docs: ReadSignal<Vec<(DocId, String)>>,
    pub current_doc: ReadSignal<Option<DocId>>,
    pub set_current_doc: WriteSignal<Option<DocId>>,
    pub tree_nodes: ReadSignal<Vec<FileNode>>,
    pub on_doc_select: Callback<DocId>,
    pub on_doc_create: Callback<String>,
    pub on_doc_rename: Callback<(String, String)>,
    pub on_doc_delete: Callback<String>,
    pub on_doc_copy: Callback<(String, String)>,
    pub on_doc_move: Callback<(String, String)>,
    pub search_results: ReadSignal<Vec<SearchHit>>,
    pub on_search: Callback<String>,
}
