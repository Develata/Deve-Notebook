//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-surface-switcher

use super::model::{
    mobile_surface_close_button_class, mobile_surface_close_touch_target,
    mobile_surface_current_state, mobile_surface_row_class, mobile_surface_row_touch_target,
};
use crate::components::editor_tabs::{EditorDiffTab, EditorDocumentTab, EditorTabKey};
use crate::components::icons::{FileText, SourceControl, X};
use crate::i18n::{Locale, t};
use deve_core::models::DocId;
use leptos::prelude::*;

#[component]
pub(super) fn SurfaceDocumentRow(
    tab: EditorDocumentTab,
    active_tab: Signal<Option<EditorTabKey>>,
    on_select: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let doc_id = tab.doc_id;
    let row_label_title = tab.title.clone();
    let close_title_label = tab.title.clone();
    let close_aria_label = tab.title.clone();
    view! {
        <div
            data-deve-mobile-surface-row="document"
            data-deve-mobile-surface-active=move || {
                document_row_active(active_tab.get(), doc_id).to_string()
            }
            class="flex items-center gap-1"
        >
            <button
                type="button"
                data-deve-mobile-surface-action="mobile_surface_document_row"
                data-deve-mobile-touch-target=mobile_surface_row_touch_target()
                class=move || mobile_surface_row_class(document_row_active(active_tab.get(), doc_id))
                title=tab.tooltip.clone()
                aria-label=move || {
                    surface_row_aria_label(t::common::document_tab(locale.get()), &row_label_title)
                }
                aria-current=move || mobile_surface_current_state(document_row_active(
                    active_tab.get(),
                    doc_id,
                ))
                on:click=move |_| on_select.run(())
            >
                <FileText class="h-4 w-4 shrink-0"/>
                <span class="min-w-0 flex-1 truncate text-[13px]">{tab.title.clone()}</span>
            </button>
            <button
                type="button"
                data-deve-mobile-surface-action="close_document"
                data-deve-mobile-touch-target=mobile_surface_close_touch_target()
                class=mobile_surface_close_button_class()
                title=move || {
                    surface_row_aria_label(t::common::close_tab(locale.get()), &close_title_label)
                }
                aria-label=move || {
                    surface_row_aria_label(t::common::close_tab(locale.get()), &close_aria_label)
                }
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
    active_tab: Signal<Option<EditorTabKey>>,
    on_select: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let key_for_active_attr = tab.key.clone();
    let key_for_active_class = tab.key.clone();
    let key_for_current_attr = tab.key.clone();
    let row_label_title = tab.title.clone();
    let close_title_label = tab.title.clone();
    let close_aria_label = tab.title.clone();
    view! {
        <div
            data-deve-mobile-surface-row="diff"
            data-deve-mobile-surface-active=move || {
                diff_row_active(active_tab.get(), &key_for_active_attr).to_string()
            }
            class="flex items-center gap-1"
        >
            <button
                type="button"
                data-deve-mobile-surface-action="mobile_surface_diff_row"
                data-deve-mobile-touch-target=mobile_surface_row_touch_target()
                class=move || {
                    mobile_surface_row_class(diff_row_active(
                        active_tab.get(),
                        &key_for_active_class,
                    ))
                }
                title=tab.tooltip.clone()
                aria-label=move || {
                    surface_row_aria_label(t::common::diff_tab(locale.get()), &row_label_title)
                }
                aria-current=move || mobile_surface_current_state(diff_row_active(
                    active_tab.get(),
                    &key_for_current_attr,
                ))
                on:click=move |_| on_select.run(())
            >
                <SourceControl class="h-4 w-4 shrink-0"/>
                <span class="min-w-0 flex-1 truncate text-[13px]">{tab.title.clone()}</span>
            </button>
            <button
                type="button"
                data-deve-mobile-surface-action="close_diff"
                data-deve-mobile-touch-target=mobile_surface_close_touch_target()
                class=mobile_surface_close_button_class()
                title=move || {
                    surface_row_aria_label(t::common::close_tab(locale.get()), &close_title_label)
                }
                aria-label=move || {
                    surface_row_aria_label(t::common::close_tab(locale.get()), &close_aria_label)
                }
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

fn document_row_active(active_tab: Option<EditorTabKey>, doc_id: DocId) -> bool {
    active_tab == Some(EditorTabKey::Document(doc_id))
}

fn diff_row_active(active_tab: Option<EditorTabKey>, key: &str) -> bool {
    matches!(active_tab, Some(EditorTabKey::Diff(active_key)) if active_key == key)
}

fn surface_row_aria_label(surface_label: &str, title: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        surface_label.to_string()
    } else {
        format!("{surface_label}: {title}")
    }
}

#[cfg(test)]
mod tests {
    use super::{diff_row_active, document_row_active, surface_row_aria_label};
    use crate::components::editor_tabs::EditorTabKey;
    use crate::i18n::{Locale, t};
    use deve_core::models::DocId;

    #[test]
    fn document_row_active_tracks_matching_document_key() {
        let doc_id = DocId::from_u128(7);

        assert!(document_row_active(
            Some(EditorTabKey::Document(doc_id)),
            doc_id
        ));
        assert!(!document_row_active(
            Some(EditorTabKey::Document(DocId::from_u128(8))),
            doc_id
        ));
        assert!(!document_row_active(
            Some(EditorTabKey::Diff("diff-a".into())),
            doc_id
        ));
        assert!(!document_row_active(None, doc_id));
    }

    #[test]
    fn diff_row_active_tracks_matching_diff_key() {
        assert!(diff_row_active(
            Some(EditorTabKey::Diff("diff-a".into())),
            "diff-a"
        ));
        assert!(!diff_row_active(
            Some(EditorTabKey::Diff("diff-b".into())),
            "diff-a"
        ));
        assert!(!diff_row_active(
            Some(EditorTabKey::Document(DocId::from_u128(7))),
            "diff-a"
        ));
        assert!(!diff_row_active(None, "diff-a"));
    }

    #[test]
    fn row_aria_label_includes_visible_surface_title() {
        assert_eq!(
            surface_row_aria_label(t::common::document_tab(Locale::Zh), "notes/a.md"),
            format!("{}: notes/a.md", t::common::document_tab(Locale::Zh))
        );
        assert_eq!(
            surface_row_aria_label(t::common::diff_tab(Locale::En), "src/lib.rs"),
            "Diff tab: src/lib.rs"
        );
        assert_ne!(
            surface_row_aria_label(t::common::document_tab(Locale::En), "a.md"),
            surface_row_aria_label(t::common::document_tab(Locale::En), "b.md")
        );
        assert_eq!(
            surface_row_aria_label(t::common::document_tab(Locale::En), "  "),
            t::common::document_tab(Locale::En)
        );
    }
}
