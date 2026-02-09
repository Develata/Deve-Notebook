use super::fold::FoldState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn FoldControls(fold_state: FoldState) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <div class="absolute top-2 left-2 z-10 flex items-center gap-2">
            <button
                class="diff-fold-toggle rounded border border-[var(--diff-border)] bg-[var(--diff-header-bg)] px-2 py-1 text-[11px] text-[var(--diff-muted)] hover:bg-[var(--diff-btn-hover)]"
                on:click=move |_| fold_state.toggle_folding.run(())
            >
                {move || if fold_state.folding_enabled.get() { t::diff::show_all_lines(locale.get()) } else { t::diff::fold_unchanged(locale.get()) }}
            </button>
            <label class="text-[11px] text-[var(--diff-muted)] flex items-center gap-1">
                {move || t::diff::context_lines(locale.get())}
                <select
                    name="diff-context-lines"
                    class="rounded border border-[var(--diff-border)] bg-[var(--diff-header-bg)] text-[11px] px-1 py-0.5"
                    prop:value=move || fold_state.context_lines.get().to_string()
                    on:change=move |ev| {
                        if let Ok(v) = event_target_value(&ev).parse::<usize>() {
                            fold_state.set_context_lines.run(v);
                        }
                    }
                >
                    <option value="3">"3"</option>
                    <option value="5">"5"</option>
                    <option value="8">"8"</option>
                </select>
            </label>
        </div>
    }
}
