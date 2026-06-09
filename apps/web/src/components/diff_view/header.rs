//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::icons::X;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(crate) fn mobile_diff_close_button_marker() -> &'static str {
    "diff-close-button"
}

pub(crate) fn mobile_diff_close_button_class() -> &'static str {
    "diff-close-button h-11 min-w-[44px] rounded p-2 text-[var(--diff-muted)] active:bg-[var(--diff-btn-hover)]"
}

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
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    if mobile {
        let close_filename = filename.clone();
        return view! {
            <div
                class="flex-none border-b border-[var(--diff-border)] bg-[var(--diff-header-bg)] px-3 pb-2"
                style="padding-top: env(safe-area-inset-top);"
            >
                <div class="flex h-11 items-center justify-between gap-2">
                    <div class="flex min-w-0 flex-1 items-center gap-2">
                        <span class="shrink-0 font-semibold text-[var(--diff-fg)]">
                            {move || format!("{}:", t::diff::title(locale.get()))}
                        </span>
                        <span
                            class="min-w-0 flex-1 truncate text-[var(--diff-filename)]"
                            title=close_filename
                        >
                            {filename.clone()}
                        </span>
                        <Show when=move || is_readonly>
                            <span class="shrink-0 rounded bg-[var(--diff-pill-bg)] px-2 py-0.5 text-xs text-[var(--diff-pill-fg)]">
                                {move || t::diff::read_only(locale.get())}
                            </span>
                        </Show>
                    </div>
                    <button
                        data-deve-mobile-diff-action=mobile_diff_close_button_marker()
                        class=mobile_diff_close_button_class()
                        on:click=move |_| on_close.run(())
                        title=move || t::diff::close_diff_view(locale.get())
                    >
                        <X class="w-5 h-5"/>
                    </button>
                </div>

                <div class="mt-1 flex h-10 items-center gap-2 overflow-x-auto pb-1">
                    <span class="shrink-0 rounded bg-[var(--diff-line-add)] px-1.5 py-0.5 text-[11px] text-[var(--diff-fg)]" title=move || t::diff::added(locale.get())>
                        {move || format!("+{}", added_count.get())}
                    </span>
                    <span class="shrink-0 rounded bg-[var(--diff-line-del)] px-1.5 py-0.5 text-[11px] text-[var(--diff-fg)]" title=move || t::diff::deleted(locale.get())>
                        {move || format!("-{}", deleted_count.get())}
                    </span>
                    <Show when=move || !is_readonly>
                        <button
                            class="diff-edit-toggle h-9 shrink-0 rounded border border-[var(--diff-border)] px-3 text-xs text-[var(--diff-fg)] active:bg-[var(--diff-btn-hover)]"
                            on:click=move |_| toggle_edit.run(())
                        >
                            {move || if is_editing.get() { t::diff::preview_diff(locale.get()) } else { t::diff::edit(locale.get()) }}
                        </button>
                    </Show>
                    <span class="shrink-0 rounded border border-[var(--diff-border)] px-1.5 py-0.5 text-[11px] text-[var(--diff-muted)]" title=move || t::diff::cache_state_help(locale.get())>
                        {move || if cache_hit.get() { t::diff::cache_hit(locale.get()) } else { t::diff::cache_miss(locale.get()) }}
                    </span>
                    <span class="shrink-0 rounded border border-[var(--diff-border)] px-1.5 py-0.5 text-[11px] text-[var(--diff-muted)]" title=move || t::diff::cache_ratio_help(locale.get())>
                        {move || t::diff::cache_ratio(locale.get(), cache_hit_ratio.get())}
                    </span>
                    <span class="shrink-0 rounded border border-[var(--diff-border)] px-1.5 py-0.5 text-[11px] text-[var(--diff-muted)]" title=move || t::diff::algorithm_help(locale.get())>
                        {move || t::diff::algorithm(locale.get(), &algorithm.get())}
                    </span>
                    <span class="shrink-0 rounded border border-[var(--diff-border)] px-1.5 py-0.5 text-[11px] text-[var(--diff-muted)]" title=move || t::diff::compute_ms_help(locale.get())>
                        {move || t::diff::compute_ms(locale.get(), compute_ms.get())}
                    </span>
                    <Show when=move || has_hunks.get()>
                        <button
                            class="diff-prev-hunk h-9 min-w-[40px] rounded border border-[var(--diff-border)] px-2 text-xs text-[var(--diff-fg)] active:bg-[var(--diff-btn-hover)]"
                            aria-label=move || t::diff::prev_change(locale.get())
                            title=move || t::diff::prev_change_hint(locale.get())
                            on:click=move |_| on_prev_hunk.run(())
                        >
                            "↑"
                        </button>
                        <span class="shrink-0 min-w-12 text-center text-[11px] text-[var(--diff-muted)]">{move || hunk_index_text.get()}</span>
                        <button
                            class="diff-next-hunk h-9 min-w-[40px] rounded border border-[var(--diff-border)] px-2 text-xs text-[var(--diff-fg)] active:bg-[var(--diff-btn-hover)]"
                            aria-label=move || t::diff::next_change(locale.get())
                            title=move || t::diff::next_change_hint(locale.get())
                            on:click=move |_| on_next_hunk.run(())
                        >
                            "↓"
                        </button>
                    </Show>
                </div>
            </div>
        }
        .into_any();
    }

    view! {
        <div
            class="flex-none h-10 border-b border-[var(--diff-border)] flex items-center justify-between px-4 bg-[var(--diff-header-bg)]"
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
                <span class="text-[11px] px-1.5 py-0.5 rounded border border-[var(--diff-border)] text-[var(--diff-muted)]" title=move || t::diff::cache_state_help(locale.get())>
                    {move || if cache_hit.get() { t::diff::cache_hit(locale.get()) } else { t::diff::cache_miss(locale.get()) }}
                </span>
                <span class="text-[11px] px-1.5 py-0.5 rounded border border-[var(--diff-border)] text-[var(--diff-muted)]" title=move || t::diff::cache_ratio_help(locale.get())>
                    {move || t::diff::cache_ratio(locale.get(), cache_hit_ratio.get())}
                </span>
                <span class="text-[11px] px-1.5 py-0.5 rounded border border-[var(--diff-border)] text-[var(--diff-muted)]" title=move || t::diff::algorithm_help(locale.get())>
                    {move || t::diff::algorithm(locale.get(), &algorithm.get())}
                </span>
                <span class="text-[11px] px-1.5 py-0.5 rounded border border-[var(--diff-border)] text-[var(--diff-muted)]" title=move || t::diff::compute_ms_help(locale.get())>
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
                    class="p-1 hover:bg-[var(--diff-btn-hover)] rounded text-[var(--diff-muted)]"
                    on:click=move |_| on_close.run(())
                    title=move || t::diff::close_diff_view(locale.get())
                >
                    <X class="w-5 h-5"/>
                </button>
            </div>
        </div>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::{mobile_diff_close_button_class, mobile_diff_close_button_marker};

    #[test]
    fn mobile_diff_close_button_marker_is_stable() {
        assert_eq!(mobile_diff_close_button_marker(), "diff-close-button");
        assert!(mobile_diff_close_button_class().contains("diff-close-button"));
    }

    #[test]
    fn mobile_diff_close_button_is_touch_safe() {
        let class = mobile_diff_close_button_class();

        assert!(class.contains("h-11"));
        assert!(class.contains("min-w-[44px]"));
    }
}
