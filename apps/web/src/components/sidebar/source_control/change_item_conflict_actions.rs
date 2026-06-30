//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!
use crate::components::icons::{Download, Upload};
use crate::components::sidebar::source_control::touch_target::{
    SourceControlActionTone, icon_button_class,
};
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::{ChangeEntry, ConflictResolution};
use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[component]
pub fn ChangeItemConflictActions(
    core: SourceControlContext,
    locale: RwSignal<Locale>,
    entry: ChangeEntry,
    action_busy: StoredValue<Arc<AtomicBool>>,
) -> impl IntoView {
    let entry_for_keep_fs = StoredValue::new(entry.clone());
    let entry_for_keep_ledger = StoredValue::new(entry);

    view! {
        <button
            type="button"
            class=icon_button_class(SourceControlActionTone::Warning)
            data-deve-sc-action="keep-fs"
            data-deve-mobile-touch-target="source-control-keep-fs-action"
            disabled=move || !core.can_write.get()
            title=move || t::source_control::keep_file_system(locale.get())
            aria-label=move || t::source_control::keep_file_system(locale.get())
            on:click=move |ev| {
                ev.stop_propagation();
                if action_busy.get_value().swap(true, Ordering::AcqRel) {
                    return;
                }
                core.clear_notice.run(());
                core.on_resolve_conflict.run((
                    entry_for_keep_fs.get_value(),
                    ConflictResolution::KeepFs,
                ));
            }
        >
            <Upload class="w-3.5 h-3.5" />
        </button>
        <button
            type="button"
            class=icon_button_class(SourceControlActionTone::Warning)
            data-deve-sc-action="keep-ledger"
            data-deve-mobile-touch-target="source-control-keep-ledger-action"
            disabled=move || !core.can_write.get()
            title=move || t::source_control::keep_ledger(locale.get())
            aria-label=move || t::source_control::keep_ledger(locale.get())
            on:click=move |ev| {
                ev.stop_propagation();
                if action_busy.get_value().swap(true, Ordering::AcqRel) {
                    return;
                }
                core.clear_notice.run(());
                core.on_resolve_conflict.run((
                    entry_for_keep_ledger.get_value(),
                    ConflictResolution::KeepLedger,
                ));
            }
        >
            <Download class="w-3.5 h-3.5" />
        </button>
    }
}
