//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
//! External Changes sidebar view.

mod row;

use self::row::{external_change_key, external_change_row};
use crate::components::icons::{Check, ChevronRight};
use crate::hooks::use_core::ExternalChangesContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

#[component]
pub fn ExternalChangesView() -> impl IntoView {
    let core = expect_context::<ExternalChangesContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let read_block = core.read_block;

    Effect::new({
        let core = core.clone();
        move |_| {
            if !should_request_external_changes(
                core.current_repo_id.get().is_some(),
                core.pending_branch_switch.get().is_some(),
                core.pending_repo_switch.get().is_some(),
                read_block.get().is_some(),
            ) {
                return;
            }
            core.on_get_changes.run(());
        }
    });

    let core_for_apply_title = core.clone();
    let core_for_apply_disabled = core.clone();
    let core_for_apply_click = core.clone();

    view! {
        <div
            class="h-full w-full bg-sidebar flex flex-col font-sans select-none overflow-hidden text-[13px] text-primary"
            data-deve-external-changes-view="true"
        >
            <div class="flex min-h-10 items-center justify-between gap-2 border-b border-default px-3">
                <div class="min-w-0 truncate text-[11px] font-bold uppercase tracking-normal">
                    {move || t::external_changes::title(locale.get())}
                </div>
                <button
                    type="button"
                    class="inline-flex h-11 shrink-0 items-center gap-1 rounded border border-border px-3 text-[11px] font-medium text-primary hover:bg-hover disabled:cursor-not-allowed disabled:opacity-50 md:h-7 md:px-2"
                    data-deve-external-apply="true"
                    title=move || apply_title(locale.get(), &core_for_apply_title)
                    disabled=move || !can_apply_to_ledger(&core_for_apply_disabled)
                    on:click=move |_| core_for_apply_click.on_apply_to_ledger.run(())
                >
                    <Check class="h-3.5 w-3.5" />
                    <span class="max-w-28 truncate">{move || t::external_changes::apply_to_ledger(locale.get())}</span>
                </button>
            </div>
            <div class="flex-1 overflow-y-auto">
                {move || {
                    let staged = core.staged_changes.get();
                    let unstaged = core.unstaged_changes.get();
                    if staged.is_empty() && unstaged.is_empty() {
                        view! {
                            <div class="px-3 py-6 text-center text-xs text-muted">
                                {t::external_changes::no_changes(locale.get())}
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <ExternalChangesSection
                                title=t::external_changes::staged(locale.get()).to_string()
                                entries=staged
                                is_staged=true
                                core=core.clone()
                                locale
                            />
                            <ExternalChangesSection
                                title=t::external_changes::pending(locale.get()).to_string()
                                entries=unstaged
                                is_staged=false
                                core=core.clone()
                                locale
                            />
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

fn should_request_external_changes(
    has_repo: bool,
    branch_switching: bool,
    repo_switching: bool,
    read_blocked: bool,
) -> bool {
    has_repo && !branch_switching && !repo_switching && !read_blocked
}

#[component]
fn ExternalChangesSection(
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

fn external_section_key(is_staged: bool) -> &'static str {
    if is_staged { "staged" } else { "pending" }
}

fn external_section_panel_id(is_staged: bool) -> &'static str {
    if is_staged {
        "external-changes-staged-panel"
    } else {
        "external-changes-pending-panel"
    }
}

fn can_apply_to_ledger(core: &ExternalChangesContext) -> bool {
    let staged = core.staged_changes.get();
    let unstaged = core.unstaged_changes.get();
    can_apply_to_ledger_state(core.can_write.get(), &staged, &unstaged)
}

fn apply_title(locale: Locale, core: &ExternalChangesContext) -> String {
    let staged = core.staged_changes.get();
    let unstaged = core.unstaged_changes.get();

    if can_apply_to_ledger_state(core.can_write.get(), &staged, &unstaged) {
        return t::external_changes::apply_to_ledger(locale).to_string();
    }
    if external_changes_have_overlap(&staged, &unstaged) {
        return t::external_changes::overlap_blocked(locale).to_string();
    }
    t::external_changes::apply_to_ledger_disabled(locale).to_string()
}

fn can_apply_to_ledger_state(
    can_write: bool,
    staged: &[ChangeEntry],
    unstaged: &[ChangeEntry],
) -> bool {
    can_write && !staged.is_empty() && !external_changes_have_overlap(staged, unstaged)
}

fn external_changes_have_overlap(staged: &[ChangeEntry], unstaged: &[ChangeEntry]) -> bool {
    staged
        .iter()
        .chain(unstaged.iter())
        .any(|entry| entry.has_conflict)
}

#[cfg(test)]
mod tests {
    use super::{
        can_apply_to_ledger_state, external_changes_have_overlap, external_section_key,
        external_section_panel_id,
    };
    use deve_core::source_control::{ChangeEntry, ChangeStatus};

    fn entry(path: &str, has_conflict: bool) -> ChangeEntry {
        ChangeEntry {
            path: path.into(),
            renamed_from: None,
            doc_id: None,
            status: ChangeStatus::Modified,
            has_conflict,
            domain: Default::default(),
            base_seq: None,
            target_seq: None,
        }
    }

    #[test]
    fn external_changes_sections_use_stable_local_ids() {
        assert_eq!(external_section_key(true), "staged");
        assert_eq!(external_section_key(false), "pending");
        assert_eq!(
            external_section_panel_id(true),
            "external-changes-staged-panel"
        );
        assert_eq!(
            external_section_panel_id(false),
            "external-changes-pending-panel"
        );
    }

    #[test]
    fn external_changes_section_headers_are_accessible_toggles() {
        let source = include_str!("external_changes.rs");

        assert!(source.contains(concat!("data-deve-", "external-section-toggle")));
        assert!(source.contains(concat!("class=\"flex ", "h-11 w-full")));
        assert!(source.contains(concat!("md:", "h-7")));
        assert!(source.contains(concat!(
            "aria-expanded=move || ",
            "expanded.get().to_string()"
        )));
        assert!(source.contains(concat!("aria-controls=", "panel_id")));
        assert!(source.contains(concat!("data-deve-", "external-section-body")));
        assert!(source.contains(concat!("hidden=move || ", "!expanded.get()")));
        assert!(!source.contains(concat!("data-deve-", "sc-section-toggle")));
    }

    #[test]
    fn apply_to_ledger_fails_closed_when_any_external_change_overlaps() {
        let staged = vec![entry("clean.md", false)];
        let unstaged = vec![entry("overlap.md", true)];

        assert!(external_changes_have_overlap(&staged, &unstaged));
        assert!(!can_apply_to_ledger_state(true, &staged, &unstaged));
    }
}
