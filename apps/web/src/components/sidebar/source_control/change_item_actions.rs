//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 12_source_control_ui#source-control-vscode-reference-contract
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChangeItemActionSurface {
    ConfirmedLedger,
    Staged,
    Conflict,
    Workspace,
}

fn change_item_action_surface(
    entry: &ChangeEntry,
    is_staged: bool,
    has_conflict: bool,
) -> ChangeItemActionSurface {
    if entry.domain == ChangeDomain::ConfirmedLedger {
        ChangeItemActionSurface::ConfirmedLedger
    } else if is_staged {
        ChangeItemActionSurface::Staged
    } else if has_conflict {
        ChangeItemActionSurface::Conflict
    } else {
        ChangeItemActionSurface::Workspace
    }
}

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
            match change_item_action_surface(&entry, is_staged, has_conflict) {
                ChangeItemActionSurface::ConfirmedLedger => {
                    if can_open_diff {
                        view! {
                            <button
                                type="button"
                                class=icon_button_class(SourceControlActionTone::Secondary)
                                data-deve-sc-action="open-diff"
                                data-deve-mobile-touch-target="source-control-confirmed-open-diff-action"
                                disabled=move || {
                                    !can_open_change_item_diff(
                                        core.current_repo_id.get().is_some(),
                                        core.pending_branch_switch.get().is_some(),
                                        core.pending_repo_switch.get().is_some(),
                                        core.read_block.get().is_some(),
                                    )
                                }
                                title=move || t::source_control::open_diff(locale.get())
                                aria-label=move || t::source_control::open_diff(locale.get())
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    core.on_get_doc_diff.run(entry_for_open.get_value());
                                }
                            >
                                <ExternalLink class="w-3.5 h-3.5" />
                            </button>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }
                }
                ChangeItemActionSurface::Staged => view! {
                    <button
                        type="button"
                        class=icon_button_class(SourceControlActionTone::Secondary)
                        data-deve-sc-action="unstage"
                        data-deve-mobile-touch-target="source-control-unstage-action"
                        disabled=move || !core.can_write.get()
                        title=move || t::source_control::unstage_changes(locale.get())
                        aria-label=move || t::source_control::unstage_changes(locale.get())
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
                }.into_any(),
                ChangeItemActionSurface::Conflict => {
                    view! { <ChangeItemConflictActions core=core.clone() locale entry=entry.clone() action_busy /> }
                        .into_any()
                }
                ChangeItemActionSurface::Workspace => view! {
                    <ChangeItemWorkspaceActions
                        core=core.clone()
                        locale
                        entry=entry.clone()
                        can_open_diff
                        action_busy
                    />
                }.into_any(),
            }
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::{ChangeItemActionSurface, change_item_action_surface};
    use deve_core::source_control::{ChangeDomain, ChangeEntry, ChangeStatus};

    fn entry(domain: ChangeDomain) -> ChangeEntry {
        ChangeEntry {
            path: "notes/a.md".into(),
            renamed_from: None,
            doc_id: None,
            status: ChangeStatus::Modified,
            has_conflict: false,
            domain,
            base_seq: None,
            target_seq: None,
        }
    }

    #[test]
    fn confirmed_ledger_rows_only_use_confirmed_action_surface() {
        let entry = entry(ChangeDomain::ConfirmedLedger);

        assert_eq!(
            change_item_action_surface(&entry, false, false),
            ChangeItemActionSurface::ConfirmedLedger
        );
        assert_eq!(
            change_item_action_surface(&entry, true, true),
            ChangeItemActionSurface::ConfirmedLedger
        );
    }

    #[test]
    fn non_confirmed_rows_keep_existing_action_surfaces() {
        let entry = entry(ChangeDomain::WorkingDirectory);

        assert_eq!(
            change_item_action_surface(&entry, true, false),
            ChangeItemActionSurface::Staged
        );
        assert_eq!(
            change_item_action_surface(&entry, false, true),
            ChangeItemActionSurface::Conflict
        );
        assert_eq!(
            change_item_action_surface(&entry, false, false),
            ChangeItemActionSurface::Workspace
        );
    }
}
