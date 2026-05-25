//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::sidebar::item::FileTreeItem;
use crate::components::sidebar::tree::build_file_tree;
use crate::hooks::use_core::DocContext;
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use deve_core::tree::FileNode;
use leptos::prelude::*;

#[component]
pub(super) fn ExplorerTree(locale: RwSignal<Locale>, doc: DocContext) -> impl IntoView {
    let tree_nodes = Memo::new(move |_| visible_tree_nodes(doc.tree_nodes.get(), doc.docs.get()));

    view! {
        <div class="flex-1 overflow-y-auto py-2">
            {move || {
                let nodes = tree_nodes.get();
                if nodes.is_empty() {
                    view! {
                        <div class="flex flex-col items-center justify-center h-32 text-muted text-sm italic select-none">
                            {move || t::sidebar::no_docs(locale.get())}
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <For
                            each=move || nodes.clone()
                            key=|node| node.path.clone()
                            children=move |node| {
                                view! {
                                    <div class="relative">
                                        <FileTreeItem node=node.clone() depth=0 />
                                    </div>
                                }
                            }
                        />
                    }
                    .into_any()
                }
            }}
        </div>
    }
}

fn visible_tree_nodes(tree_nodes: Vec<FileNode>, docs: Vec<(DocId, String)>) -> Vec<FileNode> {
    if !tree_nodes.is_empty() || docs.is_empty() {
        tree_nodes
    } else {
        // Repo/bootstrap flows deliver DocList before TreeUpdate, so a temporary
        // tree rebuild from docs is an expected recovery path rather than a warning.
        build_file_tree(docs)
    }
}

#[cfg(test)]
mod tests;
