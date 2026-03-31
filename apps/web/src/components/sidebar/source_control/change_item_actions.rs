use crate::components::icons::*;
use crate::components::sidebar::source_control::change_item_conflict_actions::ChangeItemConflictActions;
use crate::components::sidebar::source_control::change_item_workspace_actions::ChangeItemWorkspaceActions;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
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
    let entry_for_unstage = StoredValue::new(entry.clone());

    view! {
        {move || {
            if core.write_block.get().is_some() {
                view! {}.into_any()
            } else if is_staged {
                view! {
                    <button
                        class="p-0.5 hover:bg-active rounded text-secondary"
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
