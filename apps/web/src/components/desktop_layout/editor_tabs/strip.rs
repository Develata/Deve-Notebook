//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip

use crate::components::editor_tabs::{EditorDiffTab, EditorDocumentTab, EditorTabKey};
use crate::components::icons::{FileText, SourceControl, X};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use leptos::prelude::*;

pub(crate) fn tab_button_class(active: bool) -> &'static str {
    if active {
        "h-9 min-w-[120px] max-w-[240px] border-r border-default border-t-2 border-t-accent bg-editor text-primary"
    } else {
        "h-9 min-w-[120px] max-w-[220px] border-r border-default border-t-2 border-t-transparent bg-panel text-secondary hover:bg-hover hover:text-primary"
    }
}

#[cfg(test)]
mod tests {
    use super::tab_button_class;

    #[test]
    fn active_tab_class_has_accent_top_border() {
        assert!(tab_button_class(true).contains("border-t-accent"));
        assert!(tab_button_class(false).contains("border-t-transparent"));
    }
}

#[component]
pub(crate) fn EditorTabStrip(
    doc_tabs: ReadSignal<Vec<EditorDocumentTab>>,
    diff_tabs: ReadSignal<Vec<EditorDiffTab>>,
    active_tab: Signal<Option<EditorTabKey>>,
    on_select_document: Callback<DocId>,
    on_select_diff: Callback<DiffSessionWire>,
    on_close_document: Callback<DocId>,
    on_close_diff: Callback<String>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let has_tabs =
        Signal::derive(move || !doc_tabs.get().is_empty() || !diff_tabs.get().is_empty());

    view! {
        <Show when=move || has_tabs.get()>
            <div class="flex-none h-9 bg-panel border-b border-default overflow-x-auto overflow-y-hidden" role="tablist">
                <div class="flex h-9 min-w-max items-end">
                    <For each=move || doc_tabs.get() key=|tab| tab.doc_id children=move |tab| {
                        let doc_id = tab.doc_id;
                        let select_doc = on_select_document;
                        let close_doc = on_close_document;
                        let active = Signal::derive(move || active_tab.get() == Some(EditorTabKey::Document(doc_id)));
                        view! {
                            <div class=move || tab_button_class(active.get()) role="tab" aria-selected=move || active.get().to_string()>
                                <div class="flex h-full min-w-0 items-center">
                                    <button
                                        type="button"
                                        class="flex h-full min-w-0 flex-1 items-center gap-2 px-3 text-left"
                                        title=tab.tooltip.clone()
                                        aria-label=move || t::common::document_tab(locale.get())
                                        on:click=move |_| select_doc.run(doc_id)
                                    >
                                        <FileText class="h-3.5 w-3.5 shrink-0"/>
                                        <span class="min-w-0 truncate text-[13px]">{tab.title.clone()}</span>
                                    </button>
                                    <button
                                        type="button"
                                        class="mr-1 flex h-6 w-6 shrink-0 items-center justify-center rounded text-muted hover:bg-hover hover:text-primary"
                                        title=move || t::common::close_tab(locale.get())
                                        aria-label=move || t::common::close_tab(locale.get())
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            close_doc.run(doc_id);
                                        }
                                    >
                                        <X class="h-3.5 w-3.5"/>
                                    </button>
                                </div>
                            </div>
                        }
                    }/>
                    <For each=move || diff_tabs.get() key=|tab| tab.key.clone() children=move |tab| {
                        let key = tab.key.clone();
                        let key_for_close = tab.key.clone();
                        let session = tab.session.clone();
                        let select_diff = on_select_diff;
                        let close_diff = on_close_diff;
                        let active = Signal::derive(move || active_tab.get() == Some(EditorTabKey::Diff(key.clone())));
                        view! {
                            <div class=move || tab_button_class(active.get()) role="tab" aria-selected=move || active.get().to_string()>
                                <div class="flex h-full min-w-0 items-center">
                                    <button
                                        type="button"
                                        class="flex h-full min-w-0 flex-1 items-center gap-2 px-3 text-left"
                                        title=tab.tooltip.clone()
                                        aria-label=move || t::common::diff_tab(locale.get())
                                        on:click=move |_| select_diff.run(session.clone())
                                    >
                                        <SourceControl class="h-3.5 w-3.5 shrink-0"/>
                                        <span class="min-w-0 truncate text-[13px]">{tab.title.clone()}</span>
                                    </button>
                                    <button
                                        type="button"
                                        class="mr-1 flex h-6 w-6 shrink-0 items-center justify-center rounded text-muted hover:bg-hover hover:text-primary"
                                        title=move || t::common::close_tab(locale.get())
                                        aria-label=move || t::common::close_tab(locale.get())
                                        on:click=move |ev| {
                                            ev.stop_propagation();
                                            close_diff.run(key_for_close.clone());
                                        }
                                    >
                                        <X class="h-3.5 w-3.5"/>
                                    </button>
                                </div>
                            </div>
                        }
                    }/>
                </div>
            </div>
        </Show>
    }
}
