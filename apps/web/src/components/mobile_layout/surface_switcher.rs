//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

mod model;
mod rows;

use crate::components::editor_tabs::{EditorDiffTab, EditorDocumentTab, EditorTabKey};
use crate::components::icons::{ChevronDown, FileText, SourceControl, X};
use crate::hooks::use_core::diff_session::DiffSessionWire;
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use leptos::prelude::*;

use self::model::{
    mobile_surface_expanded_state, mobile_surface_sheet_visible, mobile_surface_summary,
    mobile_surface_switcher_button_class,
};
use self::rows::{SurfaceDiffRow, SurfaceDocumentRow};

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
                    aria-haspopup="dialog"
                    aria-expanded=move || mobile_surface_expanded_state(sheet_visible.get())
                    on:click=move |_| set_open.set(true)
                >
                    {move || {
                        match summary.get().map(|item| item.kind) {
                            Some("diff") => view! { <SourceControl class="h-4 w-4 shrink-0"/> }.into_any(),
                            _ => view! { <FileText class="h-4 w-4 shrink-0"/> }.into_any(),
                        }
                    }}
                    <span class="min-w-0 flex-1 truncate text-[13px] font-medium">
                        {move || {
                            summary
                                .get()
                                .map(|item| {
                                    item.title
                                        .unwrap_or_else(|| t::common::open_tabs(locale.get()).to_string())
                                })
                                .unwrap_or_default()
                        }}
                    </span>
                    <span class="shrink-0 rounded bg-muted px-2 py-0.5 text-[11px] text-secondary">
                        {move || {
                            let count = summary.get().map(|item| item.total_count).unwrap_or(0);
                            format!("{} {}", count, t::common::open_tabs(locale.get()))
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
            <section
                data-deve-mobile-surface-sheet="open"
                role="dialog"
                class="fixed inset-x-0 bottom-0 z-[var(--z-modal)] max-h-[72vh] overflow-hidden rounded-t-lg border border-default bg-panel shadow-lg"
                style="padding-bottom: env(safe-area-inset-bottom);"
                aria-label=move || t::common::switch_open_tabs(locale.get())
            >
                <div class="flex h-12 items-center justify-between border-b border-default px-3">
                    <span class="text-sm font-semibold text-primary">
                        {move || t::common::switch_open_tabs(locale.get())}
                    </span>
                    <button
                        type="button"
                        data-deve-mobile-surface-action="close_sheet"
                        class="flex h-11 min-w-[44px] items-center justify-center rounded-md text-muted active:bg-hover"
                        title=move || t::common::close(locale.get())
                        aria-label=move || t::common::close(locale.get())
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
                            view! {
                                <SurfaceDocumentRow
                                    tab=tab
                                    active_tab=active_tab
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
                            let key_for_close = tab.key.clone();
                            let session = tab.session.clone();
                            let select_diff = on_select_diff;
                            let close_diff = on_close_diff;
                            view! {
                                <SurfaceDiffRow
                                    tab=tab
                                    active_tab=active_tab
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
            </section>
        </Show>
    }
}
