//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 08_ui_design_01_web#web-layout-persistence
//!
use super::line_render::LineRender;
use super::model::hunk_fold::UnifiedRow;
use super::state::ComputePhase;
use super::unified::ChunkWindow;
use crate::i18n::{Locale, t};
use leptos::html;
use leptos::prelude::*;

pub(crate) fn first_viewport_rendered_marker(
    compute_state: ComputePhase,
    visible_rows: usize,
) -> Option<&'static str> {
    matches!(
        compute_state,
        ComputePhase::PartialReady | ComputePhase::Ready
    )
    .then_some(visible_rows)
    .filter(|rows| *rows > 0)
    .map(|_| "diff-first-viewport-rendered")
}

#[component]
pub fn UnifiedPane(
    lines: Memo<Vec<UnifiedRow>>,
    visible_lines: Memo<Vec<UnifiedRow>>,
    window: Memo<ChunkWindow>,
    unified_ref: NodeRef<html::Div>,
    set_scroll_top: WriteSignal<i32>,
    set_viewport_h: WriteSignal<i32>,
    compute_state: ReadSignal<ComputePhase>,
    on_expand_fold: Callback<usize>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    view! {
        <div
            class="diff-unified-viewport flex-1 flex overflow-auto"
            node_ref=unified_ref
            on:scroll=move |_| {
                if let Some(el) = unified_ref.get() {
                    set_scroll_top.set(el.scroll_top());
                    set_viewport_h.set(el.client_height());
                }
            }
        >
            <div class="w-12 flex-none bg-[var(--diff-gutter-bg)] text-right pr-3 text-[var(--diff-gutter-fg)] select-none py-1 border-r border-[var(--diff-border)]">
                <div style=move || format!("height: {}px", window.get().spacer_before_px())></div>
                <For
                    each=move || visible_lines.get()
                    key=|item| item.key()
                    children=|item| {
                        match item {
                            UnifiedRow::Line(line) => view! { <div class="h-[20px] leading-[20px]">{line.num.map(|n| n.to_string()).unwrap_or_default()}</div> }.into_any(),
                            UnifiedRow::Fold { .. } => view! { <div class="h-[20px] leading-[20px] text-[var(--diff-muted)]">"..."</div> }.into_any(),
                        }
                    }
                />
                <div
                    style=move || {
                        let total = lines.get().len();
                        format!("height: {}px", window.get().spacer_after_px(total))
                    }
                ></div>
            </div>
            <div class="flex-1 min-w-0 py-1 bg-[var(--diff-bg)] select-text">
                <div style=move || format!("height: {}px", window.get().spacer_before_px())></div>
                <For
                    each=move || visible_lines.get()
                    key=|item| item.key()
                    children=move |item| {
                        let row_key = item.key();
                        match item {
                            UnifiedRow::Line(line) => view! {
                                <div data-anchor-key=row_key class=format!("h-[20px] leading-[20px] whitespace-pre px-2 {}", line.class)>
                                    <LineRender content=line.content ranges=line.word_ranges kind=line.kind />
                                </div>
                            }
                            .into_any(),
                            UnifiedRow::Fold { id, hidden_count } => view! {
                                <button
                                    data-anchor-key=row_key
                                    class="diff-fold-row h-[20px] leading-[20px] whitespace-pre px-2 w-full text-left text-[11px] text-[var(--diff-muted)] hover:bg-[var(--diff-btn-hover)]"
                                    on:click=move |_| on_expand_fold.run(id)
                                >
                                    {move || t::diff::folded_lines(locale.get(), hidden_count)}
                                </button>
                            }
                            .into_any(),
                        }
                    }
                />
                <div
                    style=move || {
                        let total = lines.get().len();
                        format!("height: {}px", window.get().spacer_after_px(total))
                    }
                ></div>
                <Show when=move || {
                    first_viewport_rendered_marker(compute_state.get(), visible_lines.get().len()).is_some()
                }>
                    <div
                        class="diff-first-viewport-rendered hidden"
                        data-deve-diff-first-viewport=move || {
                            first_viewport_rendered_marker(compute_state.get(), visible_lines.get().len()).unwrap_or("")
                        }
                    ></div>
                </Show>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::first_viewport_rendered_marker;
    use crate::components::diff_view::state::ComputePhase;

    #[test]
    fn diff_first_viewport_marker_requires_ready_visible_rows() {
        assert_eq!(
            first_viewport_rendered_marker(ComputePhase::Ready, 1),
            Some("diff-first-viewport-rendered")
        );
        assert_eq!(first_viewport_rendered_marker(ComputePhase::Ready, 0), None);
        assert_eq!(
            first_viewport_rendered_marker(ComputePhase::PartialReady, 1),
            Some("diff-first-viewport-rendered")
        );
        assert_eq!(
            first_viewport_rendered_marker(ComputePhase::Computing, 1),
            None
        );
    }
}
