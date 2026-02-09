use super::line_render::LineRender;
use super::model::hunk_fold::UnifiedRow;
use super::state::ComputePhase;
use super::unified::ChunkWindow;
use crate::i18n::{Locale, t};
use leptos::html;
use leptos::prelude::*;

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
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
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
                <Show when=move || compute_state.get() == ComputePhase::Ready && !visible_lines.get().is_empty()>
                    <div class="diff-first-viewport-rendered hidden"></div>
                </Show>
            </div>
        </div>
    }
}
