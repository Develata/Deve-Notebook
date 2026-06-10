//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!
use crate::components::icons::{ExternalLink, Plus, RotateCcw};
use crate::components::sidebar::source_control::change_item_read_gate::can_open_change_item_diff;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[component]
pub fn ChangeItemWorkspaceActions(
    core: SourceControlContext,
    locale: RwSignal<Locale>,
    entry: ChangeEntry,
    can_open_diff: bool,
    action_busy: StoredValue<Arc<AtomicBool>>,
) -> impl IntoView {
    let entry_for_open = StoredValue::new(entry.clone());
    let entry_for_discard = StoredValue::new(entry.clone());
    let entry_for_stage = StoredValue::new(entry);

    view! {
        <Show when=move || can_open_diff>
            <button
                class="p-0.5 hover:bg-active rounded text-secondary"
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
        </Show>
        <button
            class="p-0.5 hover:bg-active rounded text-secondary"
            disabled=move || !core.can_write.get()
            title=move || t::source_control::discard_changes(locale.get())
            on:click=move |ev| {
                ev.stop_propagation();
                if action_busy.get_value().swap(true, Ordering::AcqRel) {
                    return;
                }
                core.clear_notice.run(());
                core.on_discard_file.run(entry_for_discard.get_value());
            }
        >
            <RotateCcw class="w-3.5 h-3.5" />
        </button>
        <button
            class="p-0.5 hover:bg-active rounded text-secondary"
            disabled=move || !core.can_write.get()
            title=move || t::source_control::stage_changes(locale.get())
            on:click=move |ev| {
                ev.stop_propagation();
                if action_busy.get_value().swap(true, Ordering::AcqRel) {
                    return;
                }
                core.clear_notice.run(());
                core.on_stage_file.run(entry_for_stage.get_value());
            }
        >
            <Plus class="w-3.5 h-3.5" />
        </button>
    }
}
