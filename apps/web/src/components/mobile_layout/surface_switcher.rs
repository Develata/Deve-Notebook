//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

mod model;
mod rows;

use crate::components::editor_tabs::{EditorDiffTab, EditorDocumentTab, EditorTabKey};
use crate::components::focus_scope;
use crate::components::icons::{ChevronDown, FileText, SourceControl, X};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use leptos::prelude::*;

use self::model::{
    mobile_surface_close_button_class, mobile_surface_sheet_visible, mobile_surface_summary,
    mobile_surface_switcher_button_class,
};
use self::rows::{SurfaceDiffRow, SurfaceDocumentRow};

pub(crate) fn mobile_surface_close_sheet_marker() -> &'static str {
    "close_sheet"
}

pub(crate) fn mobile_surface_sheet_label_id() -> &'static str {
    "mobile-surface-sheet-title"
}

pub(crate) fn mobile_surface_sheet_role() -> &'static str {
    "dialog"
}

pub(crate) fn mobile_surface_escape_closes_sheet(key: &str) -> bool {
    key == "Escape"
}

pub(crate) fn mobile_surface_kind_label(locale: Locale, kind: &str) -> &'static str {
    match kind {
        "diff" => t::common::diff_surface(locale),
        _ => t::common::document_surface(locale),
    }
}

#[component]
pub fn MobileSurfaceSwitcher(
    doc_tabs: ReadSignal<Vec<EditorDocumentTab>>,
    diff_tabs: ReadSignal<Vec<EditorDiffTab>>,
    active_tab: Signal<Option<EditorTabKey>>,
    open: ReadSignal<bool>,
    set_open: WriteSignal<bool>,
    drawer_open: Signal<bool>,
    on_select_document: Callback<DocId>,
    on_select_diff: Callback<DiffSessionWire>,
    on_close_document: Callback<DocId>,
    on_close_diff: Callback<String>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let panel_ref = NodeRef::<leptos::html::Div>::new();
    let close_button_ref = NodeRef::<leptos::html::Button>::new();
    let has_tabs =
        Signal::derive(move || !doc_tabs.get().is_empty() || !diff_tabs.get().is_empty());
    let summary = Signal::derive(move || {
        mobile_surface_summary(active_tab.get(), &doc_tabs.get(), &diff_tabs.get())
    });
    let sheet_visible = Signal::derive(move || {
        mobile_surface_sheet_visible(open.get(), drawer_open.get(), has_tabs.get())
    });

    Effect::new(move |_| {
        if drawer_open.get() || !has_tabs.get() {
            set_open.set(false);
        }
    });

    focus_scope::attach_modal_focus_restore_effect(move || sheet_visible.get(), close_button_ref);

    view! {
        <Show when=move || has_tabs.get()>
            <div
                data-deve-mobile-surface-switcher="summary"
                class="flex-none border-b border-default bg-panel px-2 py-1"
            >
                <button
                    type="button"
                    data-deve-mobile-surface-action="open_switcher"
                    data-deve-mobile-touch-target="surface_switcher"
                    data-deve-mobile-surface-kind=move || {
                        summary.get().map(|item| item.kind).unwrap_or("none")
                    }
                    class=mobile_surface_switcher_button_class()
                    title=move || t::common::switch_open_tabs(locale.get())
                    aria-label=move || t::common::switch_open_tabs(locale.get())
                    on:click=move |_| set_open.set(true)
                >
                    {move || {
                        match summary.get().map(|item| item.kind) {
                            Some("diff") => view! { <SourceControl class="h-4 w-4 shrink-0"/> }.into_any(),
                            _ => view! { <FileText class="h-4 w-4 shrink-0"/> }.into_any(),
                        }
                    }}
                    <span class="min-w-0 flex-1 truncate text-[13px] font-medium">
                        {move || summary.get().map(|item| item.title).unwrap_or_default()}
                    </span>
                    <span
                        data-deve-mobile-surface-kind-label=move || {
                            let locale = locale.get();
                            summary
                                .get()
                                .map(|item| mobile_surface_kind_label(locale, item.kind))
                                .unwrap_or("")
                        }
                        class="shrink-0 rounded border border-default px-1.5 py-0.5 text-[11px] font-medium text-secondary"
                    >
                        {move || {
                            let locale = locale.get();
                            summary
                                .get()
                                .map(|item| mobile_surface_kind_label(locale, item.kind))
                                .unwrap_or("")
                        }}
                    </span>
                    <span
                        title=move || t::common::open_tabs(locale.get())
                        class="shrink-0 rounded bg-muted px-2 py-0.5 text-[11px] text-secondary"
                    >
                        {move || {
                            let count = summary.get().map(|item| item.total_count).unwrap_or(0);
                            t::common::open_tabs_count(locale.get(), count)
                        }}
                    </span>
                    <ChevronDown class="h-4 w-4 shrink-0 text-muted"/>
                </button>
            </div>
        </Show>

        <Show when=move || sheet_visible.get()>
            <div
                data-deve-mobile-surface-overlay="true"
                class="fixed inset-0 z-[var(--z-overlay)] bg-black/20"
                on:click=move |_| set_open.set(false)
            ></div>
            <div
                node_ref=panel_ref
                role=mobile_surface_sheet_role()
                aria-modal="true"
                aria-labelledby=mobile_surface_sheet_label_id()
                tabindex="-1"
                data-deve-mobile-surface-sheet="open"
                class="fixed inset-x-0 bottom-0 z-[var(--z-modal)] max-h-[72vh] overflow-hidden rounded-t-lg border border-default bg-panel shadow-lg"
                style="padding-bottom: env(safe-area-inset-bottom);"
                on:keydown=move |ev| {
                    if focus_scope::handle_focus_trap_keydown(&ev, panel_ref) {
                        return;
                    }
                    if mobile_surface_escape_closes_sheet(&ev.key()) {
                        ev.prevent_default();
                        set_open.set(false);
                    }
                }
            >
                <div class="flex h-12 items-center justify-between border-b border-default px-3">
                    <span
                        id=mobile_surface_sheet_label_id()
                        class="text-sm font-semibold text-primary"
                    >
                        {move || t::common::switch_open_tabs(locale.get())}
                    </span>
                    <button
                        node_ref=close_button_ref
                        type="button"
                        data-deve-mobile-surface-action=mobile_surface_close_sheet_marker()
                        data-deve-mobile-touch-target=mobile_surface_close_sheet_marker()
                        class=mobile_surface_close_button_class()
                        title=move || t::common::close_tab_switcher(locale.get())
                        aria-label=move || t::common::close_tab_switcher(locale.get())
                        on:click=move |_| set_open.set(false)
                    >
                        <X class="h-4 w-4"/>
                    </button>
                </div>
                <div class="max-h-[calc(72vh-3rem)] overflow-y-auto px-2 py-2">
                    <Show when=move || !doc_tabs.get().is_empty()>
                        <div class="px-2 pb-1 pt-2 text-[11px] font-semibold uppercase text-muted">
                            {move || t::common::documents(locale.get())}
                        </div>
                        <For each=move || doc_tabs.get() key=|tab| tab.doc_id children=move |tab| {
                            let doc_id = tab.doc_id;
                            let select_doc = on_select_document;
                            let close_doc = on_close_document;
                            let active = Signal::derive(move || active_tab.get() == Some(EditorTabKey::Document(doc_id)));
                            view! {
                                <SurfaceDocumentRow
                                    tab=tab
                                    active=active
                                    on_select=Callback::new(move |_| {
                                        set_open.set(false);
                                        select_doc.run(doc_id);
                                    })
                                    on_close=Callback::new(move |_| {
                                        set_open.set(false);
                                        close_doc.run(doc_id);
                                    })
                                />
                            }
                        }/>
                    </Show>
                    <Show when=move || !diff_tabs.get().is_empty()>
                        <div class="px-2 pb-1 pt-2 text-[11px] font-semibold uppercase text-muted">
                            {move || t::common::diffs(locale.get())}
                        </div>
                        <For each=move || diff_tabs.get() key=|tab| tab.key.clone() children=move |tab| {
                            let key = tab.key.clone();
                            let key_for_close = tab.key.clone();
                            let session = tab.session.clone();
                            let select_diff = on_select_diff;
                            let close_diff = on_close_diff;
                            let active = Signal::derive(move || active_tab.get() == Some(EditorTabKey::Diff(key.clone())));
                            view! {
                                <SurfaceDiffRow
                                    tab=tab
                                    active=active
                                    on_select=Callback::new(move |_| {
                                        set_open.set(false);
                                        select_diff.run(session.clone());
                                    })
                                    on_close=Callback::new(move |_| {
                                        set_open.set(false);
                                        close_diff.run(key_for_close.clone());
                                    })
                                />
                            }
                        }/>
                    </Show>
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        mobile_surface_close_sheet_marker, mobile_surface_escape_closes_sheet,
        mobile_surface_kind_label, mobile_surface_sheet_label_id, mobile_surface_sheet_role,
    };
    use crate::i18n::{Locale, t};

    #[test]
    fn mobile_surface_kind_label_is_localized() {
        assert_eq!(
            mobile_surface_kind_label(Locale::En, "document"),
            t::common::document_surface(Locale::En)
        );
        assert_eq!(
            mobile_surface_kind_label(Locale::Zh, "document"),
            t::common::document_surface(Locale::Zh)
        );
        assert_eq!(
            mobile_surface_kind_label(Locale::En, "diff"),
            t::common::diff_surface(Locale::En)
        );
        assert_eq!(
            mobile_surface_kind_label(Locale::Zh, "diff"),
            t::common::diff_surface(Locale::Zh)
        );
        assert_eq!(
            mobile_surface_kind_label(Locale::Zh, "unknown"),
            t::common::document_surface(Locale::Zh)
        );
    }

    #[test]
    fn mobile_surface_close_sheet_marker_is_stable() {
        assert_eq!(mobile_surface_close_sheet_marker(), "close_sheet");
    }

    #[test]
    fn mobile_surface_sheet_dialog_semantics_are_stable() {
        assert_eq!(mobile_surface_sheet_role(), "dialog");
        assert_eq!(
            mobile_surface_sheet_label_id(),
            "mobile-surface-sheet-title"
        );
    }

    #[test]
    fn mobile_surface_sheet_escape_key_closes() {
        assert!(mobile_surface_escape_closes_sheet("Escape"));
        assert!(!mobile_surface_escape_closes_sheet("Tab"));
        assert!(!mobile_surface_escape_closes_sheet("Enter"));
    }
}
