//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use super::model::{mobile_surface_close_button_class, mobile_surface_row_class};
use crate::components::editor_tabs::{EditorDiffTab, EditorDocumentTab};
use crate::components::icons::{FileText, SourceControl, X};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(super) fn mobile_surface_document_row_marker() -> &'static str {
    "mobile_surface_document_row"
}

pub(super) fn mobile_surface_diff_row_marker() -> &'static str {
    "mobile_surface_diff_row"
}

pub(super) fn mobile_surface_close_document_marker() -> &'static str {
    "close_document"
}

pub(super) fn mobile_surface_close_diff_marker() -> &'static str {
    "close_diff"
}

#[component]
pub(super) fn SurfaceDocumentRow(
    tab: EditorDocumentTab,
    active: Signal<bool>,
    on_select: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let title = tab.title.clone();
    let row_label_title = title.clone();
    let close_title = title.clone();
    let close_label_title = title.clone();
    let tooltip = tab.tooltip.clone();
    view! {
        <div
            data-deve-mobile-surface-row="document"
            data-deve-mobile-surface-active=move || active.get().to_string()
            class="flex items-center gap-1"
        >
            <button
                type="button"
                data-deve-mobile-surface-action=mobile_surface_document_row_marker()
                data-deve-mobile-touch-target=mobile_surface_document_row_marker()
                class=move || mobile_surface_row_class(active.get())
                title=tooltip
                aria-label=move || t::common::document_tab_named(locale.get(), &row_label_title)
                on:click=move |_| on_select.run(())
            >
                <FileText class="h-4 w-4 shrink-0"/>
                <span class="min-w-0 flex-1 truncate text-[13px]">{title}</span>
            </button>
            <button
                type="button"
                data-deve-mobile-surface-action=mobile_surface_close_document_marker()
                data-deve-mobile-touch-target=mobile_surface_close_document_marker()
                class=mobile_surface_close_button_class()
                title=move || t::common::close_tab_named(locale.get(), &close_title)
                aria-label=move || t::common::close_tab_named(locale.get(), &close_label_title)
                on:click=move |ev| {
                    ev.stop_propagation();
                    on_close.run(());
                }
            >
                <X class="h-4 w-4"/>
            </button>
        </div>
    }
}

#[component]
pub(super) fn SurfaceDiffRow(
    tab: EditorDiffTab,
    active: Signal<bool>,
    on_select: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let title = tab.title.clone();
    let row_label_title = title.clone();
    let close_title = title.clone();
    let close_label_title = title.clone();
    let tooltip = tab.tooltip.clone();
    view! {
        <div
            data-deve-mobile-surface-row="diff"
            data-deve-mobile-surface-active=move || active.get().to_string()
            class="flex items-center gap-1"
        >
            <button
                type="button"
                data-deve-mobile-surface-action=mobile_surface_diff_row_marker()
                data-deve-mobile-touch-target=mobile_surface_diff_row_marker()
                class=move || mobile_surface_row_class(active.get())
                title=tooltip
                aria-label=move || t::common::diff_tab_named(locale.get(), &row_label_title)
                on:click=move |_| on_select.run(())
            >
                <SourceControl class="h-4 w-4 shrink-0"/>
                <span class="min-w-0 flex-1 truncate text-[13px]">{title}</span>
            </button>
            <button
                type="button"
                data-deve-mobile-surface-action=mobile_surface_close_diff_marker()
                data-deve-mobile-touch-target=mobile_surface_close_diff_marker()
                class=mobile_surface_close_button_class()
                title=move || t::common::close_tab_named(locale.get(), &close_title)
                aria-label=move || t::common::close_tab_named(locale.get(), &close_label_title)
                on:click=move |ev| {
                    ev.stop_propagation();
                    on_close.run(());
                }
            >
                <X class="h-4 w-4"/>
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        mobile_surface_close_diff_marker, mobile_surface_close_document_marker,
        mobile_surface_diff_row_marker, mobile_surface_document_row_marker,
    };
    use crate::i18n::{Locale, t};

    #[test]
    fn mobile_surface_touch_target_markers_are_stable() {
        assert_eq!(
            mobile_surface_document_row_marker(),
            "mobile_surface_document_row"
        );
        assert_eq!(mobile_surface_diff_row_marker(), "mobile_surface_diff_row");
        assert_eq!(mobile_surface_close_document_marker(), "close_document");
        assert_eq!(mobile_surface_close_diff_marker(), "close_diff");
    }

    #[test]
    fn mobile_surface_accessible_labels_include_titles() {
        let title = "notes/alpha.md";
        let zh_document_label = t::common::document_tab_named(Locale::Zh, title);
        assert!(zh_document_label.starts_with(t::common::document_tab(Locale::Zh)));
        assert!(zh_document_label.ends_with(title));
        assert_eq!(
            t::common::diff_tab_named(Locale::En, title),
            "Diff tab: notes/alpha.md"
        );
        let zh_close_label = t::common::close_tab_named(Locale::Zh, title);
        assert!(zh_close_label.starts_with(t::common::close_tab(Locale::Zh)));
        assert!(zh_close_label.ends_with(title));
    }
}
