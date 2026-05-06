//! plan_ref:
//!   - 08_ui_design_03_mobile#mobile-responsive-layout
//!   - 15_release#runtime-observability
//!
use super::footer_status::StatusView;
use crate::components::branch_switcher::BranchSwitcher;
use crate::components::icons::{ChevronDown, ChevronUp};
use crate::editor::EditorStats;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

const COL_PLACEHOLDER: &str = "-";

pub(super) fn summary_fields_class() -> &'static str {
    "flex-1 min-w-0 flex items-center gap-1 whitespace-nowrap overflow-x-auto scrollbar-none"
}

pub(super) fn mobile_summary_stat_label(
    _is_narrow: bool,
    _compact: &'static str,
    full: &'static str,
) -> &'static str {
    full
}

pub(super) fn bottom_bar_after_toggle(expanded: bool) -> bool {
    !expanded
}

#[cfg(test)]
pub(super) fn collapsed_summary_fields(is_narrow: bool, locale: Locale) -> Vec<&'static str> {
    vec![
        t::bottom_bar::branch(locale),
        t::bottom_bar::ready(locale),
        mobile_summary_stat_label(is_narrow, "W", t::bottom_bar::words(locale)),
        mobile_summary_stat_label(is_narrow, "L", t::bottom_bar::lines(locale)),
        mobile_summary_stat_label(is_narrow, "C", t::bottom_bar::col(locale)),
    ]
}

#[component]
pub fn FooterSummaryRow(
    core: CoreState,
    locale: RwSignal<Locale>,
    is_narrow: ReadSignal<bool>,
    expanded: ReadSignal<bool>,
    set_expanded: WriteSignal<bool>,
    displayed_stats: Signal<EditorStats>,
) -> impl IntoView {
    let stat_label = move |compact: &'static str, full: fn(Locale) -> &'static str| {
        let locale = locale;
        move || mobile_summary_stat_label(is_narrow.get(), compact, full(locale.get())).to_string()
    };

    view! {
        <div
            data-deve-mobile-bottom-bar-row="summary"
            data-deve-mobile-bottom-bar-single-line="true"
            class="flex items-center gap-1.5 whitespace-nowrap overflow-hidden"
        >
            <div
                data-deve-mobile-bottom-bar-fields-overflow="scroll-x"
                class=summary_fields_class()
            >
                <div data-deve-mobile-bottom-bar-field="branch" class="shrink-0 flex items-center gap-1">
                    <span class="text-[10px] text-muted">{move || t::bottom_bar::branch(locale.get())}</span>
                    <BranchSwitcher compact=true />
                </div>
                <div data-deve-mobile-bottom-bar-field="status" class="shrink-0 px-1.5 h-6 rounded-md bg-sidebar border border-default flex items-center">
                    {move || view! { <StatusView core=core.clone() locale=locale /> }}
                </div>
                <div data-deve-mobile-bottom-bar-field="words" class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted">
                    <span>{stat_label("W", t::bottom_bar::words)}</span>
                    <span class="font-mono text-primary">{move || displayed_stats.get().words}</span>
                </div>
                <div data-deve-mobile-bottom-bar-field="lines" class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted">
                    <span>{stat_label("L", t::bottom_bar::lines)}</span>
                    <span class="font-mono text-primary">{move || displayed_stats.get().lines}</span>
                </div>
                <div
                    data-deve-mobile-bottom-bar-field="col"
                    data-deve-mobile-bottom-bar-col-source="placeholder"
                    class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted"
                >
                    <span>{stat_label("C", t::bottom_bar::col)}</span>
                    <span class="font-mono text-primary">{COL_PLACEHOLDER}</span>
                </div>
            </div>

            <button
                data-deve-mobile-bottom-bar-toggle="bottom_bar_toggle"
                class="h-11 min-w-[44px] p-1.5 rounded-md active:bg-hover flex items-center justify-center"
                title=move || t::bottom_bar::toggle_status_details(locale.get())
                aria-label=move || t::bottom_bar::toggle_status_details(locale.get())
                on:click=move |_| set_expanded.update(|v| *v = bottom_bar_after_toggle(*v))
            >
                {move || if expanded.get() {
                    view! {
                        <span class="h-8 w-8 rounded-md border border-default bg-panel text-secondary flex items-center justify-center">
                            <ChevronDown class="w-4 h-4"/>
                        </span>
                    }.into_any()
                } else {
                    view! {
                        <span class="h-8 w-8 rounded-md border border-default bg-panel text-secondary flex items-center justify-center">
                            <ChevronUp class="w-4 h-4"/>
                        </span>
                    }.into_any()
                }}
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bottom_bar_after_toggle, collapsed_summary_fields, mobile_summary_stat_label,
        summary_fields_class,
    };
    use crate::i18n::Locale;

    #[test]
    fn mobile_bottom_bar_collapsed_summary_exposes_required_fields() {
        assert_eq!(
            collapsed_summary_fields(false, Locale::En),
            vec!["Branch", "Ready", "Words", "Lines", "Col"]
        );
    }

    #[test]
    fn mobile_bottom_bar_narrow_summary_keeps_single_line_labels() {
        assert_eq!(mobile_summary_stat_label(true, "W", "Words"), "Words");
        assert_eq!(mobile_summary_stat_label(false, "W", "Words"), "Words");
    }

    #[test]
    fn mobile_bottom_bar_collapsed_fields_scroll_horizontally_without_wrapping() {
        let class = summary_fields_class();
        assert!(class.contains("whitespace-nowrap"));
        assert!(class.contains("overflow-x-auto"));
        assert!(!class.contains("overflow-hidden"));
    }

    #[test]
    fn mobile_bottom_bar_expand_toggle_flips_state() {
        assert!(bottom_bar_after_toggle(false));
        assert!(!bottom_bar_after_toggle(true));
    }
}
