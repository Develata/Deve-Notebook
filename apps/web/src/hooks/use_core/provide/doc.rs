use super::super::contexts::DocContext;
use super::super::types::CoreState;

pub(super) fn build_doc_context(state: &CoreState) -> DocContext {
    let document = &state.runtime_clients.document;
    DocContext {
        docs: document.docs,
        current_doc: document.current_doc,
        set_current_doc: document.set_current_doc,
        tree_nodes: document.tree_nodes,
        on_doc_select: document.on_doc_select,
        on_doc_create: document.on_doc_create,
        on_doc_rename: document.on_doc_rename,
        on_doc_delete: document.on_doc_delete,
        on_doc_copy: document.on_doc_copy,
        on_doc_move: document.on_doc_move,
        search_results: state.search_results,
        on_search: state.on_search,
    }
}
