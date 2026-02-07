use super::line_render::LineRender;
use super::model::UnifiedLine;
use super::unified::ChunkWindow;
use leptos::html;
use leptos::prelude::*;

#[component]
pub fn UnifiedPane(
    lines: Memo<Vec<UnifiedLine>>,
    visible_lines: Memo<Vec<UnifiedLine>>,
    window: Memo<ChunkWindow>,
    unified_ref: NodeRef<html::Div>,
    set_scroll_top: WriteSignal<i32>,
    set_viewport_h: WriteSignal<i32>,
    compute_state: ReadSignal<String>,
) -> impl IntoView {
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
                    key=|item| format!("{}{}", item.num.unwrap_or(0), item.content)
                    children=|item| view! { <div class="h-[20px] leading-[20px]">{item.num.map(|n| n.to_string()).unwrap_or_default()}</div> }
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
                    key=|item| format!("{}{}", item.num.unwrap_or(0), item.content)
                    children=|item| view! {
                        <div class=format!("h-[20px] leading-[20px] whitespace-pre px-2 {}", item.class)>
                            <LineRender content=item.content ranges=item.word_ranges kind=item.kind />
                        </div>
                    }
                />
                <div
                    style=move || {
                        let total = lines.get().len();
                        format!("height: {}px", window.get().spacer_after_px(total))
                    }
                ></div>
                <Show when=move || compute_state.get() == "ready" && !visible_lines.get().is_empty()>
                    <div class="diff-first-viewport-rendered hidden"></div>
                </Show>
            </div>
        </div>
    }
}
