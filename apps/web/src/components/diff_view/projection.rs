//! Projection-only split/unified rendering and viewport selection.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 10_rendering#large-document-runtime

use std::collections::BTreeSet;
use std::sync::Arc;

use super::projection_model::{
    DisplayLine, LINE_HEIGHT_PX, VirtualWindow, display_index_for_row, display_lines,
    validate_projection,
};
use super::projection_row::ProjectionLine;
use crate::i18n::{Locale, t};
use deve_core::source_control::diff_projection::DiffProjection;
use leptos::{html, prelude::*};

#[component]
pub(super) fn ProjectionBody(
    projection: Arc<DiffProjection>,
    force_unified: bool,
) -> impl IntoView {
    let validation_error = validate_projection(&projection).err();
    let (unified, set_unified) = signal(force_unified);
    let (context_lines, set_context_lines) = signal(5u8);
    let (folding, set_folding) = signal(true);
    let expanded = RwSignal::new(BTreeSet::<String>::new());
    let (scroll_top, set_scroll_top) = signal(0usize);
    let (viewport_height, set_viewport_height) = signal(600usize);
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let hunk_rows: Vec<u32> = projection.hunks.iter().map(|hunk| hunk.row_start).collect();
    let hunk_count = hunk_rows.len();
    let hunk_rows = StoredValue::new(hunk_rows);
    let (hunk_index, set_hunk_index) = signal(0usize);
    let projection_for_lines = projection.clone();
    let lines = Memo::new(move |_| {
        display_lines(
            &projection_for_lines,
            unified.get(),
            context_lines.get(),
            folding.get(),
            &expanded.get(),
        )
    });
    let viewport_ref = NodeRef::<html::Div>::new();
    let previous_hunk = Callback::new(move |_| {
        if hunk_count == 0 {
            return;
        }
        let next = if hunk_index.get_untracked() == 0 {
            hunk_count - 1
        } else {
            hunk_index.get_untracked() - 1
        };
        set_hunk_index.set(next);
        hunk_rows
            .with_value(|rows| scroll_to_hunk(lines, viewport_ref, rows[next], set_scroll_top));
    });
    let next_hunk = Callback::new(move |_| {
        if hunk_count == 0 {
            return;
        }
        let next = (hunk_index.get_untracked() + 1) % hunk_count;
        set_hunk_index.set(next);
        hunk_rows
            .with_value(|rows| scroll_to_hunk(lines, viewport_ref, rows[next], set_scroll_top));
    });
    let window = Memo::new(move |_| {
        lines.with(|lines| {
            VirtualWindow::for_viewport(lines.len(), scroll_top.get(), viewport_height.get())
        })
    });
    let visible = Memo::new(move |_| {
        let window = window.get();
        lines.with(|lines| lines[window.start..window.end].to_vec())
    });
    let before = Signal::derive(move || window.get().start * LINE_HEIGHT_PX);
    let after = Signal::derive(move || {
        let line_count = lines.with(Vec::len);
        line_count.saturating_sub(window.get().end) * LINE_HEIGHT_PX
    });
    let projection_for_render = projection.clone();
    let on_navigation_key = {
        let previous_hunk = previous_hunk.clone();
        let next_hunk = next_hunk.clone();
        move |event: web_sys::KeyboardEvent| {
            let Some(direction) =
                hunk_navigation_direction(&event.key(), event.alt_key(), event.shift_key())
            else {
                return;
            };
            event.prevent_default();
            if direction < 0 {
                previous_hunk.run(());
            } else {
                next_hunk.run(());
            }
        }
    };

    if let Some(error) = validation_error {
        return view! {
            <div class="flex h-full items-center justify-center p-6 text-sm text-[var(--diff-muted)]" data-deve-diff-status="invalid-projection">
                {move || t::diff::invalid_projection(locale.get(), error)}
            </div>
        }
        .into_any();
    }

    view! {
        <div class="flex h-full min-h-0 flex-col" tabindex="0" on:keydown=on_navigation_key>
            <div class="flex-none flex items-center gap-2 border-b border-[var(--diff-border)] bg-[var(--diff-header-bg)] px-3 py-1 text-[11px] text-[var(--diff-muted)]">
                <Show when=move || !force_unified>
                    <button class="rounded border border-[var(--diff-border)] px-2 py-1" on:click=move |_| set_unified.update(|value| *value = !*value)>
                        {move || if unified.get() { t::diff::split(locale.get()) } else { t::diff::unified(locale.get()) }}
                    </button>
                </Show>
                <button class="diff-fold-toggle rounded border border-[var(--diff-border)] px-2 py-1" on:click=move |_| set_folding.update(|value| *value = !*value)>
                    {move || if folding.get() { t::diff::show_all_lines(locale.get()) } else { t::diff::fold_unchanged(locale.get()) }}
                </button>
                <label class="flex items-center gap-1">
                    {move || t::diff::context_lines(locale.get())}
                    <select
                        name="diff-context-lines"
                        class="rounded border border-[var(--diff-border)] bg-[var(--diff-header-bg)] px-1 py-0.5"
                        prop:value=move || context_lines.get().to_string()
                        on:change=move |event| {
                            if let Ok(value) = event_target_value(&event).parse::<u8>()
                                && matches!(value, 3 | 5 | 8)
                            {
                                set_context_lines.set(value);
                                expanded.set(BTreeSet::new());
                            }
                        }
                    >
                        <option value="3">"3"</option>
                        <option value="5">"5"</option>
                        <option value="8">"8"</option>
                    </select>
                </label>
                <Show when=move || hunk_count != 0>
                    <button class="diff-prev-hunk rounded border border-[var(--diff-border)] px-2 py-1" aria-label=move || t::diff::prev_change(locale.get()) title=move || t::diff::prev_change_hint(locale.get()) on:click=move |_| previous_hunk.run(())>"↑"</button>
                    <span>{move || format!("{}/{}", hunk_index.get() + 1, hunk_count)}</span>
                    <button class="diff-next-hunk rounded border border-[var(--diff-border)] px-2 py-1" aria-label=move || t::diff::next_change(locale.get()) title=move || t::diff::next_change_hint(locale.get()) on:click=move |_| next_hunk.run(())>"↓"</button>
                </Show>
            </div>
            <div
                node_ref=viewport_ref
                class="flex-1 min-h-0 overflow-auto bg-[var(--diff-bg)]"
                data-deve-diff-viewport="virtualized"
                on:scroll=move |event| {
                    let element = event_target::<web_sys::HtmlElement>(&event);
                    set_scroll_top.set(element.scroll_top().max(0) as usize);
                    set_viewport_height.set(element.client_height().max(1) as usize);
                }
            >
                <div style=move || format!("height:{}px", before.get())></div>
                <For
                    each=move || visible.get()
                    key=DisplayLine::key
                    children=move |line| view! {
                        <ProjectionLine
                            projection=projection_for_render.clone()
                            line
                            expanded
                        />
                    }
                />
                <div style=move || format!("height:{}px", after.get())></div>
            </div>
        </div>
    }
    .into_any()
}

fn hunk_navigation_direction(key: &str, alt: bool, shift: bool) -> Option<i8> {
    match (key, alt, shift) {
        ("[", false, _) | ("ArrowUp", true, _) | ("F7", false, true) => Some(-1),
        ("]", false, _) | ("ArrowDown", true, _) | ("F7", false, false) => Some(1),
        _ => None,
    }
}

fn scroll_to_hunk(
    lines: Memo<Vec<DisplayLine>>,
    viewport_ref: NodeRef<html::Div>,
    row_start: u32,
    set_scroll_top: WriteSignal<usize>,
) {
    let top =
        lines.with_untracked(|lines| display_index_for_row(lines, row_start)) * LINE_HEIGHT_PX;
    set_scroll_top.set(top);
    if let Some(viewport) = viewport_ref.get_untracked() {
        viewport.set_scroll_top(top.min(i32::MAX as usize) as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::hunk_navigation_direction;

    #[test]
    fn hunk_navigation_keyboard_contract_is_stable() {
        assert_eq!(hunk_navigation_direction("F7", false, false), Some(1));
        assert_eq!(hunk_navigation_direction("F7", false, true), Some(-1));
        assert_eq!(hunk_navigation_direction("ArrowDown", true, false), Some(1));
        assert_eq!(hunk_navigation_direction("ArrowUp", true, false), Some(-1));
        assert_eq!(hunk_navigation_direction("]", false, false), Some(1));
        assert_eq!(hunk_navigation_direction("[", false, false), Some(-1));
        assert_eq!(hunk_navigation_direction("ArrowDown", false, false), None);
    }
}
