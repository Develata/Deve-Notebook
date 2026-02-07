use super::line_render::LineRender;
use super::model::LineView;
use leptos::html;
use leptos::prelude::*;

#[component]
pub fn SplitPane(
    diff_result: Memo<(Vec<LineView>, Vec<LineView>)>,
    left_ref: NodeRef<html::Div>,
    right_ref: NodeRef<html::Div>,
    syncing_left: ReadSignal<bool>,
    set_syncing_left: WriteSignal<bool>,
    syncing_right: ReadSignal<bool>,
    set_syncing_right: WriteSignal<bool>,
    is_editing: ReadSignal<bool>,
    content: ReadSignal<String>,
    set_content: WriteSignal<String>,
) -> impl IntoView {
    view! {
        <>
            <div
                class="flex-1 flex overflow-auto border-r border-[var(--diff-border)]"
                node_ref=left_ref
                on:scroll=move |_| {
                    if syncing_right.get() {
                        return;
                    }
                    if let (Some(left), Some(right)) = (left_ref.get(), right_ref.get()) {
                        set_syncing_left.set(true);
                        right.set_scroll_top(left.scroll_top());
                        set_syncing_left.set(false);
                    }
                }
            >
                <div class="w-10 flex-none bg-[var(--diff-gutter-bg)] text-right pr-3 text-[var(--diff-gutter-fg)] select-none py-1 border-r border-[var(--diff-border)]">
                    <For
                        each=move || diff_result.get().0
                        key=|item| format!("{}{}", item.num.unwrap_or(0), item.content)
                        children=|item| view! { <div class="h-[20px] leading-[20px]">{item.num.map(|n| n.to_string()).unwrap_or_default()}</div> }
                    />
                </div>
                <div class="flex-1 min-w-0 py-1 bg-[var(--diff-bg)]">
                    <For
                        each=move || diff_result.get().0
                        key=|item| format!("{}{}", item.num.unwrap_or(0), item.content)
                        children=|item| view! {
                            <div class=format!("h-[20px] leading-[20px] whitespace-pre px-2 {}", item.class)>
                                <LineRender content=item.content ranges=item.word_ranges kind=item.kind />
                            </div>
                        }
                    />
                </div>
            </div>

            <div
                class="flex-1 flex overflow-auto relative"
                node_ref=right_ref
                on:scroll=move |_| {
                    if syncing_left.get() {
                        return;
                    }
                    if let (Some(left), Some(right)) = (left_ref.get(), right_ref.get()) {
                        set_syncing_right.set(true);
                        left.set_scroll_top(right.scroll_top());
                        set_syncing_right.set(false);
                    }
                }
            >
                {move || if is_editing.get() {
                    view! {
                        <textarea
                            name="diff-edit-desktop"
                            class="w-full h-full p-2 resize-none outline-none font-mono text-[13px] bg-[var(--diff-bg)] text-[var(--diff-fg)] border-none"
                            prop:value=move || content.get()
                            on:input=move |ev| set_content.set(event_target_value(&ev))
                        ></textarea>
                    }
                    .into_any()
                } else {
                    view! {
                        <div class="flex w-full min-h-full">
                            <div class="w-10 flex-none bg-[var(--diff-gutter-bg)] text-right pr-3 text-[var(--diff-gutter-fg)] select-none py-1 border-r border-[var(--diff-border)]">
                                <For
                                    each=move || diff_result.get().1
                                    key=|item| format!("{}{}", item.num.unwrap_or(0), item.content)
                                    children=|item| view! { <div class="h-[20px] leading-[20px]">{item.num.map(|n| n.to_string()).unwrap_or_default()}</div> }
                                />
                            </div>
                            <div class="flex-1 min-w-0 py-1 bg-[var(--diff-bg)] select-text">
                                <For
                                    each=move || diff_result.get().1
                                    key=|item| format!("{}{}", item.num.unwrap_or(0), item.content)
                                    children=|item| view! {
                                        <div class=format!("h-[20px] leading-[20px] whitespace-pre px-2 {}", item.class)>
                                            <LineRender content=item.content ranges=item.word_ranges kind=item.kind />
                                        </div>
                                    }
                                />
                            </div>
                        </div>
                    }
                    .into_any()
                }}
            </div>
        </>
    }
}
