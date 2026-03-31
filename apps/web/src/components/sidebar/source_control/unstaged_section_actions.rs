use crate::components::icons::{Plus, RotateCcw};
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

#[component]
pub fn UnstagedSectionActions(
    count: usize,
    bulk_busy: ReadSignal<bool>,
    set_bulk_busy: WriteSignal<bool>,
    entries_for_stage: StoredValue<Vec<ChangeEntry>>,
    entries_for_discard: StoredValue<Vec<ChangeEntry>>,
) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let write_block = core.write_block;

    view! {
        <div class="flex items-center gap-2">
            <div
                class="hidden group-hover:!flex items-center gap-1 text-primary"
                on:click=move |e| e.stop_propagation()
            >
                <Show when=move || write_block.get().is_none()>
                    <button
                        class="p-0.5 hover:bg-active rounded"
                        title=move || t::source_control::discard_all_changes(locale.get())
                        disabled=move || bulk_busy.get() || !core.can_write.get()
                        on:click=move |_| {
                            set_bulk_busy.set(true);
                            core.clear_notice.run(());
                            for entry in entries_for_discard.get_value() {
                                core.on_discard_file.run(entry);
                            }
                        }
                    >
                        <RotateCcw class="w-3.5 h-3.5" />
                    </button>
                    <button
                        class="p-0.5 hover:bg-active rounded"
                        title=move || t::source_control::stage_all_changes(locale.get())
                        disabled=move || bulk_busy.get() || !core.can_write.get()
                        on:click=move |_| {
                            set_bulk_busy.set(true);
                            core.clear_notice.run(());
                            core.on_stage_files.run(entries_for_stage.get_value());
                        }
                    >
                        <Plus class="w-3.5 h-3.5" />
                    </button>
                </Show>
            </div>
            <span class="bg-badge-count text-on-accent text-[10px] px-1.5 rounded-full min-w-[16px] text-center">
                {count}
            </span>
        </div>
    }
}
