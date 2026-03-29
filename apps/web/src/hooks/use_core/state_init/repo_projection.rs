use deve_core::tree::FileNode;
use leptos::prelude::*;

#[derive(Clone, Copy)]
pub(super) struct RepoProjectionSignals {
    pub shadow_repos: ReadSignal<Vec<String>>,
    pub set_shadow_repos: WriteSignal<Vec<String>>,
    pub shadow_list_request_id: ReadSignal<Option<String>>,
    pub set_shadow_list_request_id: WriteSignal<Option<String>>,
    pub repo_list: ReadSignal<Vec<String>>,
    pub set_repo_list: WriteSignal<Vec<String>>,
    pub repo_list_request_id: ReadSignal<Option<String>>,
    pub set_repo_list_request_id: WriteSignal<Option<String>>,
    pub doc_list_request_id: ReadSignal<Option<String>>,
    pub set_doc_list_request_id: WriteSignal<Option<String>>,
    pub tree_request_id: ReadSignal<Option<String>>,
    pub set_tree_request_id: WriteSignal<Option<String>>,
    pub tree_nodes: ReadSignal<Vec<FileNode>>,
    pub set_tree_nodes: WriteSignal<Vec<FileNode>>,
}

pub(super) fn init_repo_projection_signals() -> RepoProjectionSignals {
    let (shadow_repos, set_shadow_repos) = signal(Vec::new());
    let (shadow_list_request_id, set_shadow_list_request_id) = signal(None::<String>);
    let (repo_list, set_repo_list) = signal(Vec::new());
    let (repo_list_request_id, set_repo_list_request_id) = signal(None::<String>);
    let (doc_list_request_id, set_doc_list_request_id) = signal(None::<String>);
    let (tree_request_id, set_tree_request_id) = signal(None::<String>);
    let (tree_nodes, set_tree_nodes) = signal(Vec::<FileNode>::new());

    RepoProjectionSignals {
        shadow_repos,
        set_shadow_repos,
        shadow_list_request_id,
        set_shadow_list_request_id,
        repo_list,
        set_repo_list,
        repo_list_request_id,
        set_repo_list_request_id,
        doc_list_request_id,
        set_doc_list_request_id,
        tree_request_id,
        set_tree_request_id,
        tree_nodes,
        set_tree_nodes,
    }
}
