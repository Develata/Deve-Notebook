//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

mod model;
mod rows;

pub(crate) use self::model::mobile_surface_sheet_visible;

use crate::components::editor_tabs::{EditorDiffTab, EditorDocumentTab, EditorTabKey};
use crate::components::focus_scope::{
    attach_modal_focus_restore_effect_with_fallback, handle_focus_trap_keydown,
};
use crate::components::icons::{ChevronDown, FileText, SourceControl, X};
use crate::i18n::{Locale, t};
use crate::runtime::source_control_client::diff_session::DiffSessionWire;
use deve_core::models::DocId;
use leptos::prelude::*;

use self::model::{
    mobile_surface_close_button_class, mobile_surface_close_touch_target,
    mobile_surface_expanded_state, mobile_surface_summary, mobile_surface_summary_badge_text,
    mobile_surface_summary_title_class, mobile_surface_switcher_button_class,
    mobile_surface_switcher_next_open, mobile_surface_switcher_touch_target,
    mobile_surface_type_label_marker,
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
    let trigger_ref = NodeRef::<leptos::html::Button>::new();
    let close_ref = NodeRef::<leptos::html::Button>::new();
    let panel_ref = NodeRef::<leptos::html::Div>::new();
    attach_modal_focus_restore_effect_with_fallback(
        move || sheet_visible.get(),
        close_ref,
        trigger_ref,
    );

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
                    node_ref=trigger_ref
                    type="button"
                    data-deve-mobile-surface-action="open_switcher"
                    data-deve-mobile-touch-target=mobile_surface_switcher_touch_target()
                    data-deve-mobile-surface-kind=move || {
                        summary.get().map(|item| item.kind).unwrap_or("none")
                    }
                    class=mobile_surface_switcher_button_class()
                    title=move || t::common::switch_open_tabs(locale.get())
                    aria-label=move || t::common::switch_open_tabs(locale.get())
                    aria-haspopup="dialog"
                    aria-expanded=move || mobile_surface_expanded_state(sheet_visible.get())
                    on:click=move |_| {
                        set_open.set(mobile_surface_switcher_next_open(
                            open.get_untracked(),
                            drawer_open.get_untracked(),
                            has_tabs.get_untracked(),
                        ));
                    }
                >
                    {move || {
                        match summary.get().map(|item| item.kind) {
                            Some("diff") => view! { <SourceControl class="h-4 w-4 shrink-0"/> }.into_any(),
                            _ => view! { <FileText class="h-4 w-4 shrink-0"/> }.into_any(),
                        }
                    }}
                    <span class=move || {
                        let has_title = summary
                            .get()
                            .is_some_and(|item| item.title.is_some());
                        mobile_surface_summary_title_class(has_title)
                    }>
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
                    <span
                        data-deve-mobile-surface-type-label=mobile_surface_type_label_marker()
                        class="shrink-0 rounded bg-muted px-2 py-0.5 text-[11px] text-secondary"
                    >
                        {move || {
                            summary
                                .get()
                                .map(|item| {
                                    mobile_surface_summary_badge_text(
                                        item.kind,
                                        item.total_count,
                                        locale.get(),
                                    )
                                })
                                .unwrap_or_default()
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
                data-deve-mobile-surface-sheet="open"
                role="dialog"
                aria-modal="true"
                class="fixed inset-x-0 bottom-0 z-[var(--z-modal)] max-h-[72vh] overflow-hidden rounded-t-lg border border-default bg-panel shadow-lg"
                style="padding-bottom: var(--deve-safe-area-bottom);"
                aria-label=move || t::common::switch_open_tabs(locale.get())
                on:keydown=move |ev| {
                    if ev.key() == "Escape" {
                        ev.prevent_default();
                        set_open.set(false);
                        return;
                    }
                    let _ = handle_focus_trap_keydown(&ev, panel_ref);
                }
            >
                <div class="flex h-12 items-center justify-between border-b border-default px-3">
                    <span class="text-sm font-semibold text-primary">
                        {move || t::common::switch_open_tabs(locale.get())}
                    </span>
                    <button
                        node_ref=close_ref
                        type="button"
                        data-deve-mobile-surface-action="close_sheet"
                        data-deve-mobile-touch-target=mobile_surface_close_touch_target()
                        class=mobile_surface_close_button_class()
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
            </div>
        </Show>
    }
}
