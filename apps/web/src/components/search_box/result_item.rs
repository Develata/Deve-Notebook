// apps/web/src/components/search_box/result_item.rs
//! # 搜索结果项渲染组件
//!
//! 处理单条搜索结果的渲染和交互逻辑。

use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::components::icons::{ArrowRight, File, Folder, GitBranch, Plus, Terminal};
use crate::components::search_box::logic;
use crate::components::search_box::types::{SearchAction, SearchResult};
use crate::hooks::use_core::CoreState;

/// 单条结果项，支持鼠标与键盘操作。
#[allow(clippy::too_many_arguments)]
pub fn result_item(
    idx: usize,
    item: SearchResult,
    is_sel: bool,
    selected_index: Signal<usize>,
    set_selected_index: WriteSignal<usize>,
    set_show: WriteSignal<bool>,
    set_query: WriteSignal<String>,
    input_ref: NodeRef<leptos::html::Input>,
    core: CoreState,
    set_recent_move_dirs: WriteSignal<Vec<String>>,
) -> impl IntoView {
    let is_mobile = window_width().map(|w| w <= 768).unwrap_or(false);
    let detail_text = item.detail.clone();
    let detail_text_cond = detail_text.clone();
    let is_group =
        matches!(item.action, SearchAction::Noop) && item.detail.as_deref() == Some("Group");
    let is_error =
        matches!(item.action, SearchAction::Noop) && item.detail.as_deref() == Some("Error");
    let is_selectable = logic::is_selectable(Some(&item));

    if is_group {
        let group_class = if is_mobile {
            "px-3 py-2 text-[10px] uppercase tracking-wide text-muted"
        } else {
            "px-4 py-2 text-[11px] uppercase tracking-widest text-muted"
        };
        return view! { <div class=group_class>{item.title}</div> }.into_any();
    }

    if is_error {
        let error_class = if is_mobile {
            "px-3 py-2 text-xs text-red-500"
        } else {
            "px-4 py-2 text-sm text-red-500"
        };
        return view! {
            <div class=error_class>
                {item.title}
            </div>
        }
        .into_any();
    }

    let base = if is_mobile {
        "w-full text-left px-3 py-2 rounded-lg flex items-center gap-2 group transition-colors active:bg-hover"
    } else {
        "w-full text-left px-4 py-3 rounded-lg flex items-center gap-3 group transition-colors active:bg-hover"
    };

    let action_clone = item.action.clone();
    let detail_clone = item.detail.clone();

    view! {
        <button
            class=format!(
                "{} {}",
                base,
                if is_sel && is_selectable {
                    "bg-accent-subtle text-accent"
                } else if is_selectable {
                    "text-primary hover:bg-hover"
                } else {
                    "text-muted cursor-default"
                }
            )
            on:click=move |_| {
                if !is_selectable {
                    return;
                }
                let action = item.action.clone();
                let core_clone = core.clone();
                request_animation_frame(move || {
                    logic::execute_action(
                        &action,
                        &core_clone,
                        set_show,
                        set_query,
                        set_selected_index,
                        input_ref,
                        set_recent_move_dirs,
                    );
                });
            }
            on:mousemove=move |_: MouseEvent| {
                if is_selectable && selected_index.get_untracked() != idx {
                    set_selected_index.set(idx);
                }
            }
            on:touchstart=move |_| {
                if is_selectable && selected_index.get_untracked() != idx {
                    set_selected_index.set(idx);
                }
            }
        >
            {move || if is_mobile {
                view! {}.into_any()
            } else {
                item_icon(is_sel, action_clone.clone(), detail_clone.clone()).into_any()
            }}
            {item_content(item.title.clone(), detail_text_cond, detail_text)}
            {move || if is_mobile {
                view! {}.into_any()
            } else {
                selection_arrow(is_sel).into_any()
            }}
        </button>
    }
    .into_any()
}

fn item_icon(is_sel: bool, action: SearchAction, detail: Option<String>) -> impl IntoView {
    let cls = "w-5 h-5";
    let icon_view = match action {
        SearchAction::RunCommand(_) => view! { <Terminal class=cls/> }.into_any(),
        SearchAction::SwitchBranch(_) => view! { <GitBranch class=cls/> }.into_any(),
        SearchAction::CreateDoc(_) => view! { <Plus class=cls/> }.into_any(),
        SearchAction::InsertQuery(_) => view! { <Folder class=cls/> }.into_any(),
        SearchAction::OpenDoc(_) | SearchAction::FileOp(_) | SearchAction::Noop => {
            view! { <File class=cls/> }.into_any()
        }
    };
    view! {
        <div class=format!("flex-none {}", if is_sel { "text-accent" } else { "text-muted" })>
            {icon_view}
            {move || if detail.as_deref() == Some("Error") {
                view! { <span class="text-xs font-semibold text-red-500">"!"</span> }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}

fn item_content(
    title: String,
    detail_cond: Option<String>,
    detail_text: Option<String>,
) -> impl IntoView {
    let is_mobile = window_width().map(|w| w <= 768).unwrap_or(false);
    view! {
        <div class="flex-1 truncate flex flex-col items-start gap-0.5">
            <span class="font-medium">{title}</span>
            <Show when=move || detail_cond.is_some()>
                <span class=if is_mobile { "text-[11px] opacity-60 font-mono" } else { "text-xs opacity-60 font-mono" }>
                    {detail_text.clone().unwrap()}
                </span>
            </Show>
        </div>
    }
}

fn selection_arrow(is_sel: bool) -> impl IntoView {
    view! {
        <Show when=move || is_sel>
            <ArrowRight class="w-4 h-4 text-accent opacity-0 group-hover:opacity-100 transition-opacity"/>
        </Show>
    }
}

fn window_width() -> Option<i32> {
    let window = web_sys::window()?;
    let width = window.inner_width().ok()?.as_f64()?;
    Some(width as i32)
}
