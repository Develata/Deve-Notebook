//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 15_release#runtime-observability
//!
use crate::components::sidebar::source_control::error_notice_copy as copy;
use crate::hooks::use_core::source_control_notice::SourceControlNotice;
use crate::hooks::use_core::write_gate::RepoWriteBlock;
use crate::i18n::Locale;
use leptos::prelude::*;

#[component]
pub fn ErrorNotice(
    notice: ReadSignal<Option<SourceControlNotice>>,
    block: Signal<Option<RepoWriteBlock>>,
    clear_notice: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <Show when=move || block.get().is_none() && notice.get().is_some()>
            <div class="px-4 py-3 text-sm border-b border-default bg-warning/5">
                <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                        <p class="text-primary font-medium">
                            {move || {
                                notice
                                    .get()
                                    .map(|current| copy::title(locale.get(), &current))
                                    .unwrap_or_default()
                            }}
                        </p>
                        <p class="mt-1 text-xs text-muted">
                            {move || {
                                notice
                                    .get()
                                    .map(|current| copy::hint(locale.get(), &current))
                                    .unwrap_or_default()
                            }}
                        </p>
                        <div class="mt-2 space-y-1 text-xs text-muted">
                            {move || {
                                notice
                                    .get()
                                    .map(|current| {
                                        copy::details(locale.get(), &current)
                                            .into_iter()
                                            .map(|detail| {
                                                view! {
                                                    <p class="pl-3 border-l border-warning/30">
                                                        {detail}
                                                    </p>
                                                }
                                            })
                                            .collect_view()
                                            .into_any()
                                    })
                                    .unwrap_or_else(|| view! {}.into_any())
                            }}
                        </div>
                    </div>
                    <button
                        class="text-xs text-secondary hover:text-primary"
                        on:click=move |_| clear_notice.run(())
                    >
                        {"×"}
                    </button>
                </div>
            </div>
        </Show>
    }
}
