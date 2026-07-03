//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
//! External Changes sidebar view.

mod row;

use self::row::{external_change_key, external_change_row};
use crate::components::icons::Check;
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

    if count == 0 {
        return view! {}.into_any();
    }

    let section_marker = title.clone();
    let section_title = title;

    view! {
        <section data-deve-external-section=section_marker>
            <div
                class="flex min-h-11 items-center justify-between px-3 text-[11px] font-bold uppercase text-muted md:min-h-7"
                data-deve-mobile-touch-target="external-changes-section-header"
            >
                <span class="truncate">{section_title}</span>
                <span>{count}</span>
            </div>
            <For
                each=move || entries.get_value()
                key=external_change_key
                children=move |entry| {
                    external_change_row(entry, is_staged, core.clone(), locale)
                }
            />
        </section>
    }.into_any()
}

fn can_apply_to_ledger(core: &ExternalChangesContext) -> bool {
    let staged = core.staged_changes.get();
    core.can_write.get() && !staged.is_empty() && !staged.iter().any(|entry| entry.has_conflict)
}

fn apply_title(locale: Locale, core: &ExternalChangesContext) -> String {
    if can_apply_to_ledger(core) {
        return t::external_changes::apply_to_ledger(locale).to_string();
    }
    t::external_changes::apply_to_ledger_disabled(locale).to_string()
}
