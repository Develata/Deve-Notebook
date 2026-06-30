//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::components::icons::*;
use crate::components::sidebar::source_control::change_item_conflict_actions::ChangeItemConflictActions;
use crate::components::sidebar::source_control::change_item_read_gate::can_open_change_item_diff;
use crate::components::sidebar::source_control::change_item_workspace_actions::ChangeItemWorkspaceActions;
use crate::components::sidebar::source_control::touch_target::{
    SourceControlActionTone, icon_button_class,
};
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::{ChangeDomain, ChangeEntry};
use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[component]
pub fn ChangeItemActions(
    core: SourceControlContext,
    locale: RwSignal<Locale>,
    entry: ChangeEntry,
    is_staged: bool,
    has_conflict: bool,
    can_open_diff: bool,
    action_busy: StoredValue<Arc<AtomicBool>>,
) -> impl IntoView {
    let entry_for_open = StoredValue::new(entry.clone());
    let entry_for_unstage = StoredValue::new(entry.clone());

    view! {
        {move || {
            if should_show_confirmed_open_diff_action(entry.domain, can_open_diff) {
                view! {
                    <button
                        class=icon_button_class(SourceControlActionTone::Secondary)
                        data-deve-mobile-touch-target="source-control-confirmed-open-diff-action"
                        disabled=move || {
                            !can_open_change_item_diff(
                                core.current_repo_id.get().is_some(),
                                core.pending_branch_switch.get().is_some(),
                                core.pending_repo_switch.get().is_some(),
                                core.read_block.get().is_some(),
                            )
                        }
                        title=move || t::source_control::open_file(locale.get())
                        on:click=move |ev| {
                            ev.stop_propagation();
                            core.on_get_doc_diff.run(entry_for_open.get_value());
                        }
                    >
                        <ExternalLink class="w-3.5 h-3.5" />
                    </button>
                }.into_any()
            } else if entry.domain == ChangeDomain::ConfirmedLedger {
                view! {}.into_any()
            } else if is_staged {
                view! {
                    <button
                        class=icon_button_class(SourceControlActionTone::Secondary)
                        data-deve-mobile-touch-target="source-control-unstage-action"
                        disabled=move || !core.can_write.get()
                        title=move || t::source_control::unstage_changes(locale.get())
                        on:click=move |ev| {
                            ev.stop_propagation();
                            if action_busy.get_value().swap(true, Ordering::AcqRel) {
                                return;
                            }
                            core.clear_notice.run(());
                            core.on_unstage_file.run(entry_for_unstage.get_value());
                        }
                    >
                        <Minus class="w-3.5 h-3.5" />
                    </button>
                }
                .into_any()
            } else if has_conflict {
                view! { <ChangeItemConflictActions core=core.clone() locale entry=entry.clone() action_busy /> }
                    .into_any()
            } else {
                view! {
                    <ChangeItemWorkspaceActions
                        core=core.clone()
                        locale
                        entry=entry.clone()
                        can_open_diff
                        action_busy
                    />
                }
                .into_any()
            }
        }}
    }
}

pub(crate) fn should_show_confirmed_open_diff_action(
    domain: ChangeDomain,
    can_open_diff: bool,
) -> bool {
    domain == ChangeDomain::ConfirmedLedger && can_open_diff
}

#[cfg(test)]
mod tests {
    use super::should_show_confirmed_open_diff_action;
    use deve_core::source_control::ChangeDomain;

    #[test]
    fn confirmed_ledger_open_diff_action_policy() {
        assert!(should_show_confirmed_open_diff_action(
            ChangeDomain::ConfirmedLedger,
            true
        ));
        assert!(!should_show_confirmed_open_diff_action(
            ChangeDomain::ConfirmedLedger,
            false
        ));
        assert!(!should_show_confirmed_open_diff_action(
            ChangeDomain::WorkingDirectory,
            true
        ));
    }
}
