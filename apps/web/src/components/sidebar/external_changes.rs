//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 12_source_control_ui#external-changes-sibling-view
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
//! External Changes sidebar view.

mod row;
mod section;
mod state;

use self::section::{ExternalChangesBlockedNotice, ExternalChangesSection};
use self::state::{
    ExternalChangesVisibleState, apply_title, can_apply_to_ledger, external_changes_visible_state,
    should_request_external_changes,
};
use crate::components::icons::Check;
use crate::hooks::use_core::ExternalChangesContext;
use crate::i18n::{Locale, t};
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
                    let read_block_value = read_block.get();
                    let staged = core.staged_changes.get();
                    let unstaged = core.unstaged_changes.get();
                    match external_changes_visible_state(read_block_value, staged.len(), unstaged.len()) {
                        ExternalChangesVisibleState::Blocked(block) => view! {
                            <ExternalChangesBlockedNotice block locale />
                        }.into_any(),
                        ExternalChangesVisibleState::Empty => view! {
                            <div class="px-3 py-6 text-center text-xs text-muted">
                                {t::external_changes::no_changes(locale.get())}
                            </div>
                        }.into_any(),
                        ExternalChangesVisibleState::Changes => view! {
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
                        }.into_any(),
                    }
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
#[path = "external_changes/tests.rs"]
mod tests;
