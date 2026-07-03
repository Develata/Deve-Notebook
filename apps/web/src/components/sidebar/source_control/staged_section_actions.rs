//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 09_web_thin_client_ledger#web-edit-intent
//!   - 12_source_control_ui#source-control-vscode-reference-contract
//!
use super::action_tray::SECTION_ACTION_TRAY_CLASS;
use super::resource_group_visibility::section_bulk_action_disabled;
use crate::components::icons::Minus;
use crate::components::sidebar::source_control::touch_target::{
    SourceControlActionTone, icon_button_class,
};
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

#[component]
pub fn StagedSectionActions(
    count: usize,
    bulk_busy: ReadSignal<bool>,
    set_bulk_busy: WriteSignal<bool>,
    entries_for_action: StoredValue<Vec<ChangeEntry>>,
) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let write_block = core.write_block;

    view! {
        <div class="flex items-center gap-2">
            <div
                class=SECTION_ACTION_TRAY_CLASS
                data-deve-sc-action-tray="section"
                on:click=move |e| e.stop_propagation()
            >
                <Show when=move || write_block.get().is_none()>
                    <button
                        type="button"
                        class=icon_button_class(SourceControlActionTone::Primary)
                        data-deve-sc-action="unstage-all"
                        data-deve-mobile-touch-target="source-control-unstage-all-action"
                        title=move || t::source_control::unstage_all_changes(locale.get())
                        aria-label=move || t::source_control::unstage_all_changes(locale.get())
                        disabled=move || {
                            section_bulk_action_disabled(
                                count,
                                bulk_busy.get(),
                                core.can_write.get(),
                            )
                        }
                        on:click=move |_| {
                            set_bulk_busy.set(true);
                            core.clear_notice.run(());
                            core.on_unstage_files.run(entries_for_action.get_value());
                        }
                    >
                        <Minus class="w-3.5 h-3.5" />
                    </button>
                </Show>
            </div>
            <span class="bg-badge-count text-on-accent text-[10px] px-1.5 rounded-full min-w-[16px] text-center">
                {count}
            </span>
        </div>
    }
}
