use crate::components::icons::{ChevronRight, Plus, RefreshCw};
use crate::components::sidebar::source_control::changes::Changes;
use crate::components::sidebar::source_control::commit::Commit;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[component]
pub fn ChangesPanel(expanded: RwSignal<bool>, visible: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let header_bulk_busy = StoredValue::new(Arc::new(AtomicBool::new(false)));
    let write_block = core.write_block;
    let has_unstaged_changes = move || !core.unstaged_changes.get().is_empty();

    let header_bulk_busy_reset = header_bulk_busy;
    Effect::new(move |_| {
        let _ = core.staged_changes.get();
        let _ = core.unstaged_changes.get();
        let _ = core.notice.get();
        header_bulk_busy_reset
            .get_value()
            .store(false, Ordering::Release);
    });

    view! {
        <Show when=move || visible.get() && core.active_branch.get().is_none()>
            <div class="border-t border-default">
                <div class="flex items-center px-1 py-0.5 hover:bg-hover text-[11px] font-bold text-primary uppercase group">
                    <button
                        class="min-w-0 flex-1 flex items-center text-left focus:outline-none"
                        on:click=move |_| expanded.update(|b| *b = !*b)
                    >
                        <span class=move || if expanded.get() {
                            "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform"
                        } else {
                            "w-4 h-4 flex items-center justify-center transition-transform"
                        }>
                            <ChevronRight class="w-3 h-3" />
                        </span>
                        <span class="flex-1 text-left">{move || t::source_control::changes(locale.get())}</span>
                    </button>
                    <div class="flex items-center gap-1 md:hidden md:group-hover:flex">
                        <Show when=move || write_block.get().is_none() && has_unstaged_changes()>
                            <button
                                class="p-0.5 hover:bg-active rounded disabled:opacity-50 disabled:cursor-not-allowed"
                                title=move || t::source_control::discard_all_changes(locale.get())
                                disabled=move || !core.can_write.get() || !has_unstaged_changes()
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    if header_bulk_busy.get_value().swap(true, Ordering::AcqRel) {
                                        return;
                                    }
                                    core.clear_notice.run(());
                                    core.on_discard_pending.run(());
                                }
                            >
                                <RefreshCw class="w-3.5 h-3.5" />
                            </button>
                            <button
                                class="p-0.5 hover:bg-active rounded disabled:opacity-50 disabled:cursor-not-allowed"
                                title=move || t::source_control::stage_all_changes(locale.get())
                                disabled=move || !core.can_write.get() || !has_unstaged_changes()
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    if header_bulk_busy.get_value().swap(true, Ordering::AcqRel) {
                                        return;
                                    }
                                    core.clear_notice.run(());
                                    core.on_stage_files.run(core.unstaged_changes.get_untracked());
                                }
                            >
                                <Plus class="w-3.5 h-3.5" />
                            </button>
                        </Show>
                    </div>
                </div>

                <Show when=move || expanded.get()>
                    <div>
                        <Commit />
                        <Changes />
                    </div>
                </Show>
            </div>
        </Show>
    }
}
