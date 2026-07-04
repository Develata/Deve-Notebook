//! plan_ref:
//!   - 12_source_control_ui#external-changes-sibling-view
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
//! External Changes section and blocked-state rendering.

use super::row::{external_change_key, external_change_row};
use super::state::external_changes_blocked_hint;
use crate::components::icons::ChevronRight;
use crate::hooks::use_core::ExternalChangesContext;
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

#[component]
pub(super) fn ExternalChangesBlockedNotice(
    block: RepoWriteBlock,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    view! {
        <div
            class="px-3 py-5 text-xs text-muted"
            data-deve-external-blocked="true"
            data-deve-external-block=block.label()
        >
            <p class="text-primary font-medium">
                {move || t::external_changes::blocked_title(locale.get())}
            </p>
            <p class="mt-1 leading-5">
                {move || external_changes_blocked_hint(locale.get(), block)}
            </p>
        </div>
    }
}

#[component]
pub(super) fn ExternalChangesSection(
    title: String,
    entries: Vec<ChangeEntry>,
    is_staged: bool,
    core: ExternalChangesContext,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let count = entries.len();
    let entries = StoredValue::new(entries);
    let expanded = RwSignal::new(true);

    if count == 0 {
        return view! {}.into_any();
    }

    let section_key = external_section_key(is_staged);
    let panel_id = external_section_panel_id(is_staged);
    let section_title = title;

    view! {
        <section data-deve-external-section=section_key>
            <div
                class="flex h-11 items-center px-1 text-[11px] font-bold uppercase text-muted md:h-7"
            >
                <button
                    type="button"
                    class="flex h-11 w-full min-w-0 items-center justify-between rounded-sm px-2 text-left hover:bg-hover focus:outline-none focus-visible:ring-1 focus-visible:ring-accent/40 md:h-7"
                    data-deve-mobile-touch-target="external-changes-section-header"
                    data-deve-external-section-toggle=section_key
                    aria-expanded=move || expanded.get().to_string()
                    aria-controls=panel_id
                    on:click=move |_| expanded.update(|value| *value = !*value)
                >
                    <span class="flex min-w-0 items-center">
                        <span class=move || format!(
                            "flex h-4 w-4 items-center justify-center text-primary transition-transform {}",
                            if expanded.get() { "rotate-90" } else { "" },
                        )>
                            <ChevronRight class="h-3 w-3" />
                        </span>
                        <span class="truncate">{section_title.clone()}</span>
                    </span>
                    <span class="pl-2 text-[11px] text-muted">{count}</span>
                </button>
            </div>
            <div
                id=panel_id
                data-deve-external-section-body=section_key
                hidden=move || !expanded.get()
            >
                <For
                    each=move || entries.get_value()
                    key=external_change_key
                    children=move |entry| {
                        external_change_row(entry, is_staged, core.clone(), locale)
                    }
                />
            </div>
        </section>
    }.into_any()
}

pub(super) fn external_section_key(is_staged: bool) -> &'static str {
    if is_staged { "staged" } else { "pending" }
}

pub(super) fn external_section_panel_id(is_staged: bool) -> &'static str {
    if is_staged {
        "external-changes-staged-panel"
    } else {
        "external-changes-pending-panel"
    }
}
