//! plan_ref:
//!   - 17_tech_stack#search-baseline
//!   - 11_ui_design_01_web#web-layout-persistence
//!
use crate::components::icons::{ArrowRight, File, Folder, GitBranch, Plus, Terminal};
use crate::components::search_box::types::SearchAction;
use leptos::prelude::*;

pub(super) fn group_row(title: String, is_mobile: bool) -> impl IntoView {
    let class = if is_mobile {
        "px-3 py-2 text-[10px] uppercase tracking-wide text-muted"
    } else {
        "px-4 py-2 text-[11px] uppercase tracking-widest text-muted"
    };
    view! { <div class=class>{title}</div> }
}

pub(super) fn error_row(title: String, is_mobile: bool) -> impl IntoView {
    let class = if is_mobile {
        "px-3 py-2 text-xs text-red-500"
    } else {
        "px-4 py-2 text-sm text-red-500"
    };
    view! { <div class=class>{title}</div> }
}

pub(super) fn item_icon(
    is_sel: bool,
    action: SearchAction,
    detail: Option<String>,
) -> impl IntoView {
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

pub(super) fn item_content(
    title: String,
    detail_text: Option<String>,
    is_mobile: bool,
) -> impl IntoView {
    let detail_class = if is_mobile {
        "text-[11px] opacity-60 font-mono"
    } else {
        "text-xs opacity-60 font-mono"
    };
    let detail_view = detail_text
        .map(|detail| view! { <span class=detail_class>{detail}</span> }.into_any())
        .unwrap_or_else(|| view! {}.into_any());

    view! {
        <div class="flex-1 truncate flex flex-col items-start gap-0.5">
            <span class="font-medium">{title}</span>
            {detail_view}
        </div>
    }
}

pub(super) fn selection_arrow(is_sel: bool) -> impl IntoView {
    view! {
        <Show when=move || is_sel>
            <ArrowRight class="w-4 h-4 text-accent opacity-0 group-hover:opacity-100 transition-opacity"/>
        </Show>
    }
}
