//! plan_ref:
//!   - 11_ui_design/index#editor-group-tabstrip

use crate::components::editor_tabs::{
    DropPosition, EditorDiffTab, EditorDocumentTab, EditorTabItem, EditorTabKey,
};
use crate::components::icons::{FileText, SourceControl, X};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

pub(crate) fn tab_button_class(active: bool) -> &'static str {
    if active {
        "h-9 min-w-[120px] max-w-[240px] border-r border-default border-t-2 border-t-accent bg-editor text-primary"
    } else {
        "h-9 min-w-[120px] max-w-[220px] border-r border-default border-t-2 border-t-transparent bg-panel text-secondary hover:bg-hover hover:text-primary"
    }
}

#[cfg(test)]
mod tests {
    use super::{
        diff_tab_projection, document_tab_projection, tab_button_class, trailing_blank_drop_target,
    };
    use crate::components::editor_tabs::{
        DropPosition, EditorDiffTab, EditorDocumentTab, EditorTabItem, EditorTabKey,
    };
    use deve_core::models::DocId;

    #[test]
    fn active_tab_class_has_accent_top_border() {
        assert!(tab_button_class(true).contains("border-t-accent"));
        assert!(tab_button_class(false).contains("border-t-transparent"));
    }

    #[test]
    fn trailing_blank_drop_targets_after_last_visible_tab() {
        let first = DocId::from_u128(1);
        let second = DocId::from_u128(2);
        let tabs = vec![
            EditorTabItem::Document(EditorDocumentTab {
                doc_id: first,
                title: "a.md".into(),
                tooltip: "a.md".into(),
            }),
            EditorTabItem::Document(EditorDocumentTab {
                doc_id: second,
                title: "b.md".into(),
                tooltip: "b.md".into(),
            }),
        ];

        assert_eq!(
            trailing_blank_drop_target(&tabs),
            Some((EditorTabKey::Document(second), DropPosition::After))
        );
        assert_eq!(trailing_blank_drop_target(&[]), None);
    }

    #[test]
    fn tab_projection_helpers_follow_current_ordered_items() {
        let doc_id = DocId::from_u128(1);
        let items = vec![
            EditorTabItem::Document(EditorDocumentTab {
                doc_id,
                title: "renamed.md".into(),
                tooltip: "archive/renamed.md".into(),
            }),
            EditorTabItem::Diff(EditorDiffTab {
                key: "diff-1".into(),
                title: "current.diff".into(),
                tooltip: "current.diff".into(),
                session: crate::hooks::use_core::diff_session::DiffSessionWire::new(
                    "current.diff".into(),
                    "old".into(),
                    "new".into(),
                ),
            }),
        ];

        assert_eq!(
            document_tab_projection(&items, doc_id).map(|tab| tab.tooltip),
            Some("archive/renamed.md".into())
        );
        assert_eq!(
            diff_tab_projection(&items, "diff-1").map(|tab| tab.title),
            Some("current.diff".into())
        );
    }
}

#[component]
pub(crate) fn EditorTabStrip(
    ordered_tabs: Signal<Vec<EditorTabItem>>,
    active_tab: Signal<Option<EditorTabKey>>,
    on_select_document: Callback<DocId>,
    on_select_diff: Callback<DiffSessionWire>,
    on_close_document: Callback<DocId>,
    on_close_diff: Callback<String>,
    on_reorder_tab: Callback<(EditorTabKey, EditorTabKey, DropPosition)>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let has_tabs = Signal::derive(move || !ordered_tabs.get().is_empty());
    let trailing_drop_target =
        Signal::derive(move || trailing_blank_drop_target(&ordered_tabs.get()));
    let (dragging_tab, set_dragging_tab) = signal(None::<EditorTabKey>);

    view! {
        <Show when=move || has_tabs.get()>
            <div
                class="flex-none h-9 bg-panel border-b border-default overflow-x-auto overflow-y-hidden"
                role="tablist"
                data-deve-editor-tabstrip-trailing-drop="after-last"
                on:dragover=move |ev| ev.prevent_default()
                on:drop=move |ev| {
                    ev.prevent_default();
                    if let Some(dragged) = dragging_tab.get_untracked()
                        && let Some((target, position)) = trailing_drop_target.get_untracked()
                    {
                        on_reorder_tab.run((dragged, target, position));
                    }
                    set_dragging_tab.set(None);
                }
            >
                <div class="flex h-9 min-w-max items-end">
                    <For each=move || ordered_tabs.get() key=|item| item.key().marker() children=move |item| {
                        let tab_key = item.key();
                        let active_key = tab_key.clone();
                        let drag_key = tab_key.clone();
                        let drop_target = tab_key.clone();
                        let marker = tab_key.marker();
                        let kind = tab_key.kind();
                        let active = Signal::derive(move || active_tab.get() == Some(active_key.clone()));
                        let body = match item {
                            EditorTabItem::Document(tab) => {
                                let doc_id = tab.doc_id;
                                let tab_projection = Signal::derive(move || {
                                    document_tab_projection(&ordered_tabs.get(), doc_id)
                                });
                                let select_doc = on_select_document;
                                let close_doc = on_close_document;
                                view! {
                                    <div class="flex h-full min-w-0 items-center">
                                        <button
                                            type="button"
                                            class="flex h-full min-w-0 flex-1 items-center gap-2 px-3 text-left"
                                            title=move || {
                                                tab_projection
                                                    .get()
                                                    .map(|tab| tab.tooltip)
                                                    .unwrap_or_default()
                                            }
                                            aria-label=move || t::common::document_tab(locale.get())
                                            on:click=move |_| select_doc.run(doc_id)
                                        >
                                            <FileText class="h-3.5 w-3.5 shrink-0"/>
                                            <span class="min-w-0 truncate text-[13px]">
                                                {move || {
                                                    tab_projection
                                                        .get()
                                                        .map(|tab| tab.title)
                                                        .unwrap_or_default()
                                                }}
                                            </span>
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
                                }.into_any()
                            }
                            EditorTabItem::Diff(tab) => {
                                let key_for_close = tab.key.clone();
                                let tab_projection_key = tab.key.clone();
                                let tab_projection = Signal::derive(move || {
                                    diff_tab_projection(&ordered_tabs.get(), &tab_projection_key)
                                });
                                let session = tab.session.clone();
                                let select_diff = on_select_diff;
                                let close_diff = on_close_diff;
                                view! {
                                    <div class="flex h-full min-w-0 items-center">
                                        <button
                                            type="button"
                                            class="flex h-full min-w-0 flex-1 items-center gap-2 px-3 text-left"
                                            title=move || {
                                                tab_projection
                                                    .get()
                                                    .map(|tab| tab.tooltip)
                                                    .unwrap_or_default()
                                            }
                                            aria-label=move || t::common::diff_tab(locale.get())
                                            on:click=move |_| {
                                                let session = tab_projection
                                                    .get()
                                                    .map(|tab| tab.session)
                                                    .unwrap_or_else(|| session.clone());
                                                select_diff.run(session);
                                            }
                                        >
                                            <SourceControl class="h-3.5 w-3.5 shrink-0"/>
                                            <span class="min-w-0 truncate text-[13px]">
                                                {move || {
                                                    tab_projection
                                                        .get()
                                                        .map(|tab| tab.title)
                                                        .unwrap_or_default()
                                                }}
                                            </span>
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
                                }.into_any()
                            }
                        };
                        view! {
                            <div
                                class=move || tab_button_class(active.get())
                                role="tab"
                                aria-selected=move || active.get().to_string()
                                draggable="true"
                                data-deve-editor-tab-key=marker
                                data-deve-editor-tab-kind=kind
                                on:dragstart=move |_| set_dragging_tab.set(Some(drag_key.clone()))
                                on:dragover=move |ev| ev.prevent_default()
                                on:drop=move |ev| {
                                    ev.prevent_default();
                                    ev.stop_propagation();
                                    if let Some(dragged) = dragging_tab.get_untracked() {
                                        let position = drop_position_from_drag_event(&ev);
                                        on_reorder_tab.run((dragged, drop_target.clone(), position));
                                    }
                                    set_dragging_tab.set(None);
                                }
                                on:dragend=move |_| set_dragging_tab.set(None)
                            >
                                {body}
                            </div>
                        }
                    }/>
                </div>
            </div>
        </Show>
    }
}

fn document_tab_projection(tabs: &[EditorTabItem], doc_id: DocId) -> Option<EditorDocumentTab> {
    tabs.iter().find_map(|item| match item {
        EditorTabItem::Document(tab) if tab.doc_id == doc_id => Some(tab.clone()),
        _ => None,
    })
}

fn diff_tab_projection(tabs: &[EditorTabItem], key: &str) -> Option<EditorDiffTab> {
    tabs.iter().find_map(|item| match item {
        EditorTabItem::Diff(tab) if tab.key == key => Some(tab.clone()),
        _ => None,
    })
}

fn trailing_blank_drop_target(tabs: &[EditorTabItem]) -> Option<(EditorTabKey, DropPosition)> {
    tabs.last().map(|item| (item.key(), DropPosition::After))
}

fn drop_position_from_drag_event(ev: &web_sys::DragEvent) -> DropPosition {
    let Some(target) = ev.current_target() else {
        return DropPosition::Before;
    };
    let Ok(element) = target.dyn_into::<web_sys::Element>() else {
        return DropPosition::Before;
    };
    let rect = element.get_bounding_client_rect();
    if f64::from(ev.client_x()) > rect.left() + rect.width() / 2.0 {
        DropPosition::After
    } else {
        DropPosition::Before
    }
}
