//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 18_release#runtime-observability
//!
use super::footer_read::{read_footer_signal, read_footer_value};
use super::footer_status::StatusView;
use crate::components::branch_switcher::BranchSwitcher;
use crate::components::icons::{ChevronDown, ChevronUp};
use crate::editor::EditorStats;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

const COL_PLACEHOLDER: &str = "-";

pub(super) fn summary_fields_class() -> &'static str {
    "flex-1 min-w-0 flex items-center gap-1 whitespace-nowrap overflow-hidden"
}

pub(super) fn summary_branch_field_class() -> &'static str {
    "min-w-0 flex-1 flex items-center gap-1 overflow-hidden"
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

pub(super) fn bottom_bar_expanded_state(expanded: bool) -> &'static str {
    if expanded { "true" } else { "false" }
}

pub(super) fn bottom_bar_toggle_button_class() -> &'static str {
    "h-11 min-w-[44px] p-1.5 rounded-md active:bg-hover flex items-center justify-center"
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
    locale: RwSignal<Locale>,
    is_narrow: ReadSignal<bool>,
    expanded: ReadSignal<bool>,
    set_expanded: WriteSignal<bool>,
    displayed_stats: Signal<EditorStats>,
) -> impl IntoView {
    let stat_label = move |compact: &'static str, full: fn(Locale) -> &'static str| {
        let locale = locale;
        move || {
            mobile_summary_stat_label(
                read_footer_signal(is_narrow, false),
                compact,
                full(locale.get()),
            )
            .to_string()
        }
    };

    view! {
        <div
            data-deve-mobile-bottom-bar-row="summary"
            data-deve-mobile-bottom-bar-single-line="true"
            class="flex items-center gap-1.5 whitespace-nowrap overflow-hidden"
        >
            <div
                data-deve-mobile-bottom-bar-fields-overflow="clip"
                class=summary_fields_class()
            >
                <div data-deve-mobile-bottom-bar-field="branch" class=summary_branch_field_class()>
                    <span class="shrink-0 text-[10px] text-muted">{move || t::bottom_bar::branch(locale.get())}</span>
                    <BranchSwitcher compact=true />
                </div>
                <div data-deve-mobile-bottom-bar-field="status" class="shrink-0 px-1.5 h-6 rounded-md bg-sidebar border border-default flex items-center">
                    {move || view! { <StatusView locale=locale /> }}
                </div>
                <div data-deve-mobile-bottom-bar-field="words" class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted">
                    <span>{stat_label("W", t::bottom_bar::words)}</span>
                    <span class="font-mono text-primary">{move || read_footer_value(displayed_stats, EditorStats::default()).words}</span>
                </div>
                <div data-deve-mobile-bottom-bar-field="lines" class="shrink-0 h-6 rounded-md bg-sidebar border border-default px-1.5 flex items-center gap-1 text-[10px] text-muted">
                    <span>{stat_label("L", t::bottom_bar::lines)}</span>
                    <span class="font-mono text-primary">{move || read_footer_value(displayed_stats, EditorStats::default()).lines}</span>
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
                type="button"
                data-deve-mobile-bottom-bar-toggle="bottom_bar_toggle"
                data-deve-mobile-touch-target="bottom_bar_toggle"
                class=bottom_bar_toggle_button_class()
                title=move || t::bottom_bar::toggle_status_details(locale.get())
                aria-label=move || t::bottom_bar::toggle_status_details(locale.get())
                aria-controls="deve-mobile-bottom-bar-details"
                aria-expanded=move || bottom_bar_expanded_state(read_footer_signal(expanded, false))
                on:click=move |_| set_expanded.update(|v| *v = bottom_bar_after_toggle(*v))
            >
                {move || if read_footer_signal(expanded, false) {
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
        bottom_bar_after_toggle, bottom_bar_expanded_state, bottom_bar_toggle_button_class,
        collapsed_summary_fields, mobile_summary_stat_label, summary_branch_field_class,
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
    fn mobile_bottom_bar_collapsed_fields_fit_without_horizontal_scroll() {
        let class = summary_fields_class();
        assert!(class.contains("whitespace-nowrap"));
        assert!(class.contains("overflow-hidden"));
        assert!(!class.contains("overflow-x-auto"));
    }

    #[test]
    fn mobile_bottom_bar_branch_field_shrinks_before_status_fields() {
        let class = summary_branch_field_class();
        assert!(class.contains("min-w-0"));
        assert!(class.contains("flex-1"));
        assert!(class.contains("overflow-hidden"));
        assert!(!class.contains("shrink-0"));
    }

    #[test]
    fn mobile_bottom_bar_expand_toggle_flips_state() {
        assert!(bottom_bar_after_toggle(false));
        assert!(!bottom_bar_after_toggle(true));
    }

    #[test]
    fn mobile_bottom_bar_toggle_exposes_expanded_state() {
        assert_eq!(bottom_bar_expanded_state(false), "false");
        assert_eq!(bottom_bar_expanded_state(true), "true");
    }

    #[test]
    fn mobile_touch_targets_bottom_bar_toggle_is_at_least_44px() {
        let class = bottom_bar_toggle_button_class();
        assert!(class.contains("h-11"));
        assert!(class.contains("min-w-[44px]"));
    }
}
