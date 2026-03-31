use crate::components::sidebar::item::FileTreeItem;
use crate::hooks::use_core::DocContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub(super) fn ExplorerTree(locale: RwSignal<Locale>, doc: DocContext) -> impl IntoView {
    let tree_nodes = Memo::new(move |_| doc.tree_nodes.get());

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
