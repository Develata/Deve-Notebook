use super::super::contexts::DocContext;
use super::super::types::CoreState;

pub(super) fn build_doc_context(state: &CoreState) -> DocContext {
    DocContext {
        docs: state.docs,
        current_doc: state.current_doc,
        set_current_doc: state.set_current_doc,
        tree_nodes: state.tree_nodes,
        on_doc_select: state.on_doc_select,
        on_doc_create: state.on_doc_create,
        on_doc_rename: state.on_doc_rename,
        on_doc_delete: state.on_doc_delete,
        on_doc_copy: state.on_doc_copy,
        on_doc_move: state.on_doc_move,
        search_results: state.search_results,
        on_search: state.on_search,
    }
}
