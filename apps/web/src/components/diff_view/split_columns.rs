use super::line_render::LineRender;
use super::model::split_fold::SplitRow;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn SplitLeftColumn(
    split_rows: Memo<Vec<SplitRow>>,
    on_expand_fold: Callback<usize>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <>
            <div class="w-10 flex-none bg-[var(--diff-gutter-bg)] text-right pr-3 text-[var(--diff-gutter-fg)] select-none py-1 border-r border-[var(--diff-border)]">
                <For
                    each=move || split_rows.get()
                    key=|row| row.key()
                    children=|row| match row {
                        SplitRow::Pair { left, .. } => view! { <div class="h-[20px] leading-[20px]">{left.num.map(|n| n.to_string()).unwrap_or_default()}</div> }.into_any(),
                        SplitRow::Fold { .. } => view! { <div class="h-[20px] leading-[20px] text-[var(--diff-muted)]">"..."</div> }.into_any(),
                    }
                />
            </div>
            <div class="flex-1 min-w-0 py-1 bg-[var(--diff-bg)]">
                <For
                    each=move || split_rows.get()
                    key=|row| row.key()
                    children=move |row| {
                        let row_key = row.key();
                        match row {
                        SplitRow::Pair { left, .. } => view! {
                            <div data-anchor-key=row_key class=format!("h-[20px] leading-[20px] whitespace-pre px-2 {}", left.class)>
                                <LineRender content=left.content ranges=left.word_ranges kind=left.kind />
                            </div>
                        }.into_any(),
                        SplitRow::Fold { id, hidden_count } => view! {
                            <button data-anchor-key=row_key class="diff-fold-row h-[20px] leading-[20px] whitespace-pre px-2 w-full text-left text-[11px] text-[var(--diff-muted)] hover:bg-[var(--diff-btn-hover)]" on:click=move |_| on_expand_fold.run(id)>
                                {move || t::diff::folded_lines(locale.get(), hidden_count)}
                            </button>
                        }.into_any(),
                    }}
                />
            </div>
        </>
    }
}

#[component]
pub fn SplitRightColumn(
    split_rows: Memo<Vec<SplitRow>>,
    is_editing: ReadSignal<bool>,
    content: ReadSignal<String>,
    set_content: WriteSignal<String>,
    on_expand_fold: Callback<usize>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        {move || if is_editing.get() {
            view! { <textarea name="diff-edit-desktop" class="w-full h-full p-2 resize-none outline-none font-mono text-[13px] bg-[var(--diff-bg)] text-[var(--diff-fg)] border-none" prop:value=move || content.get() on:input=move |ev| set_content.set(event_target_value(&ev))></textarea> }.into_any()
        } else {
            view! {
                <div class="flex w-full min-h-full">
                    <div class="w-10 flex-none bg-[var(--diff-gutter-bg)] text-right pr-3 text-[var(--diff-gutter-fg)] select-none py-1 border-r border-[var(--diff-border)]">
                        <For
                            each=move || split_rows.get()
                            key=|row| row.key()
                            children=|row| match row {
                                SplitRow::Pair { right, .. } => view! { <div class="h-[20px] leading-[20px]">{right.num.map(|n| n.to_string()).unwrap_or_default()}</div> }.into_any(),
                                SplitRow::Fold { .. } => view! { <div class="h-[20px] leading-[20px] text-[var(--diff-muted)]">"..."</div> }.into_any(),
                            }
                        />
                    </div>
                    <div class="flex-1 min-w-0 py-1 bg-[var(--diff-bg)] select-text">
                        <For
                            each=move || split_rows.get()
                            key=|row| row.key()
                            children=move |row| {
                                let row_key = row.key();
                                match row {
                                SplitRow::Pair { right, .. } => view! {
                                    <div data-anchor-key=row_key class=format!("h-[20px] leading-[20px] whitespace-pre px-2 {}", right.class)>
                                        <LineRender content=right.content ranges=right.word_ranges kind=right.kind />
                                    </div>
                                }.into_any(),
                                SplitRow::Fold { id, hidden_count } => view! {
                                    <button data-anchor-key=row_key class="diff-fold-row h-[20px] leading-[20px] whitespace-pre px-2 w-full text-left text-[11px] text-[var(--diff-muted)] hover:bg-[var(--diff-btn-hover)]" on:click=move |_| on_expand_fold.run(id)>
                                        {move || t::diff::folded_lines(locale.get(), hidden_count)}
                                    </button>
                                }.into_any(),
                            }}
                        />
                    </div>
                </div>
            }.into_any()
        }}
    }
}
