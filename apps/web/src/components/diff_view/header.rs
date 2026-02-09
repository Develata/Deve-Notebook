use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn DiffHeader(
    mobile: bool,
    filename: String,
    is_readonly: bool,
    is_editing: ReadSignal<bool>,
    hunk_index_text: Signal<String>,
    has_hunks: Signal<bool>,
    added_count: Signal<usize>,
    deleted_count: Signal<usize>,
    cache_hit: Signal<bool>,
    cache_hit_ratio: Signal<u32>,
    compute_ms: Signal<u32>,
    algorithm: Signal<String>,
    on_prev_hunk: Callback<()>,
    on_next_hunk: Callback<()>,
    toggle_edit: Callback<()>,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <div
            class=move || if mobile {
                "flex-none border-b border-[var(--diff-border)] flex items-center justify-between px-3 bg-[var(--diff-header-bg)]"
            } else {
                "flex-none h-10 border-b border-[var(--diff-border)] flex items-center justify-between px-4 bg-[var(--diff-header-bg)]"
            }
            style=move || if mobile {
                "padding-top: env(safe-area-inset-top); height: calc(48px + env(safe-area-inset-top));"
            } else {
                ""
            }
        >
            <div class="flex items-center gap-2 min-w-0">
                <span class="font-semibold text-[var(--diff-fg)]">{move || format!("{}:", t::diff::title(locale.get()))}</span>
                <span class="text-[var(--diff-filename)] truncate max-w-[46vw]" title=filename.clone()>{filename.clone()}</span>
                <Show when=move || is_readonly>
                    <span class="text-xs bg-[var(--diff-pill-bg)] px-2 py-0.5 rounded text-[var(--diff-pill-fg)]">
                        {move || t::diff::read_only(locale.get())}
                    </span>
                </Show>
            </div>
            <div class="flex items-center gap-2">
                <span class="text-[11px] px-1.5 py-0.5 rounded bg-[var(--diff-line-add)] text-[var(--diff-fg)]" title=move || t::diff::added(locale.get())>
                    {move || format!("+{}", added_count.get())}
                </span>
                <span class="text-[11px] px-1.5 py-0.5 rounded bg-[var(--diff-line-del)] text-[var(--diff-fg)]" title=move || t::diff::deleted(locale.get())>
                    {move || format!("-{}", deleted_count.get())}
                </span>
                <span class="text-[11px] px-1.5 py-0.5 rounded border border-[var(--diff-border)] text-[var(--diff-muted)]">
                    {move || if cache_hit.get() { t::diff::cache_hit(locale.get()) } else { t::diff::cache_miss(locale.get()) }}
                </span>
                <span class="text-[11px] px-1.5 py-0.5 rounded border border-[var(--diff-border)] text-[var(--diff-muted)]">
                    {move || t::diff::cache_ratio(locale.get(), cache_hit_ratio.get())}
                </span>
                <span class="text-[11px] px-1.5 py-0.5 rounded border border-[var(--diff-border)] text-[var(--diff-muted)]" title=move || t::diff::algorithm_help(locale.get())>
                    {move || t::diff::algorithm(locale.get(), &algorithm.get())}
                </span>
                <span class="text-[11px] px-1.5 py-0.5 rounded border border-[var(--diff-border)] text-[var(--diff-muted)]">
                    {move || t::diff::compute_ms(locale.get(), compute_ms.get())}
                </span>
                <Show when=move || has_hunks.get()>
                    <button
                        class="diff-prev-hunk h-8 px-2 border border-[var(--diff-border)] rounded text-xs hover:bg-[var(--diff-btn-hover)] text-[var(--diff-fg)]"
                        aria-label=move || t::diff::prev_change(locale.get())
                        title=move || t::diff::prev_change_hint(locale.get())
                        on:click=move |_| on_prev_hunk.run(())
                    >
                        "↑"
                    </button>
                    <span class="text-[11px] text-[var(--diff-muted)] min-w-12 text-center">{move || hunk_index_text.get()}</span>
                    <button
                        class="diff-next-hunk h-8 px-2 border border-[var(--diff-border)] rounded text-xs hover:bg-[var(--diff-btn-hover)] text-[var(--diff-fg)]"
                        aria-label=move || t::diff::next_change(locale.get())
                        title=move || t::diff::next_change_hint(locale.get())
                        on:click=move |_| on_next_hunk.run(())
                    >
                        "↓"
                    </button>
                </Show>
                <Show when=move || !is_readonly>
                    <button
                        class=move || if mobile {
                            "diff-edit-toggle h-9 px-3 border border-[var(--diff-border)] rounded text-xs active:bg-[var(--diff-btn-hover)] text-[var(--diff-fg)]"
                        } else {
                            "diff-edit-toggle px-3 py-1 border border-[var(--diff-border)] rounded text-xs hover:bg-[var(--diff-btn-hover)] text-[var(--diff-fg)]"
                        }
                        on:click=move |_| toggle_edit.run(())
                    >
                        {move || if is_editing.get() { t::diff::preview_diff(locale.get()) } else { t::diff::edit(locale.get()) }}
                    </button>
                </Show>
                <button
                    class=move || if mobile {
                        "diff-close-button h-11 min-w-11 p-2 active:bg-[var(--diff-btn-hover)] rounded text-[var(--diff-muted)]"
                    } else {
                        "p-1 hover:bg-[var(--diff-btn-hover)] rounded text-[var(--diff-muted)]"
                    }
                    on:click=move |_| on_close.run(())
                    title=move || t::diff::close_diff_view(locale.get())
                >
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                </button>
            </div>
        </div>
    }
}
