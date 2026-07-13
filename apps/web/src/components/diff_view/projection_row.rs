//! DOM rows for a validated backend diff projection.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 10_rendering#large-document-runtime

use std::collections::BTreeSet;
use std::sync::Arc;

use super::projection_model::{DisplayLine, Side};
use super::projection_text::highlighted_parts;
use crate::i18n::{Locale, t};
use deve_core::source_control::diff_projection::{
    DiffCellKind, DiffCellProjection, DiffProjection,
};
use leptos::prelude::*;

#[component]
pub(super) fn ProjectionLine(
    projection: Arc<DiffProjection>,
    line: DisplayLine,
    expanded: RwSignal<BTreeSet<String>>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    match line {
        DisplayLine::Fold { id, hidden, .. } => {
            let expand_id = id.clone();
            view! {
                <button
                    class="diff-fold-row h-[20px] w-full px-3 text-left text-[11px] leading-[20px] text-[var(--diff-muted)] hover:bg-[var(--diff-btn-hover)]"
                    data-deve-diff-fold=id
                    on:click=move |_| { expanded.update(|ids| { ids.insert(expand_id.clone()); }); }
                >{move || t::diff::folded_lines(locale.get(), hidden as usize)}</button>
            }
            .into_any()
        }
        DisplayLine::Split { row } => {
            let row = &projection.rows[row as usize];
            let hunk = row.hunk_id.map(|id| id.to_string()).unwrap_or_default();
            let left = row.left.clone();
            let right = row.right.clone();
            view! {
                <div class="grid h-[20px] grid-cols-[3rem_minmax(0,1fr)_3rem_minmax(0,1fr)] leading-[20px]" data-hunk-id=hunk>
                    <CellNumber cell=left.clone()/>
                    <CellText projection=projection.clone() cell=left left=true/>
                    <CellNumber cell=right.clone()/>
                    <CellText projection=projection cell=right left=false/>
                </div>
            }
            .into_any()
        }
        DisplayLine::Unified { row, side } => {
            let row = &projection.rows[row as usize];
            let (cell, left, prefix) = match side {
                Side::Left => (row.left.clone(), true, "-"),
                Side::Right => {
                    let prefix = if row.right.kind == DiffCellKind::Add {
                        "+"
                    } else {
                        " "
                    };
                    (row.right.clone(), false, prefix)
                }
            };
            let hunk = row.hunk_id.map(|id| id.to_string()).unwrap_or_default();
            view! {
                <div class="grid h-[20px] grid-cols-[3rem_1.5rem_minmax(0,1fr)] leading-[20px]" data-hunk-id=hunk>
                    <CellNumber cell=cell.clone()/>
                    <span class="select-none text-center text-[var(--diff-muted)]">{prefix}</span>
                    <CellText projection cell left/>
                </div>
            }
            .into_any()
        }
    }
}

#[component]
fn CellNumber(cell: DiffCellProjection) -> impl IntoView {
    view! {
        <span class="select-none border-r border-[var(--diff-border)] bg-[var(--diff-gutter-bg)] pr-2 text-right text-[var(--diff-gutter-fg)]">
            {cell.line_number.map(|number| number.to_string()).unwrap_or_default()}
        </span>
    }
}

#[component]
fn CellText(
    projection: Arc<DiffProjection>,
    cell: DiffCellProjection,
    left: bool,
) -> impl IntoView {
    let text = if cell.kind == DiffCellKind::Empty {
        ""
    } else {
        projection
            .cell_text(&cell, left)
            .expect("projection validated before row rendering")
    };
    let parts = highlighted_parts(text, &cell.word_ranges)
        .expect("UTF-16 ranges validated before row rendering");
    let row_class = match cell.kind {
        DiffCellKind::Add => "bg-[var(--diff-line-add)]",
        DiffCellKind::Delete => "bg-[var(--diff-line-del)]",
        DiffCellKind::Empty => "bg-[var(--diff-line-empty)]",
        DiffCellKind::Context => "",
    };
    let mark_class = match cell.kind {
        DiffCellKind::Add => "bg-[var(--diff-word-add)]",
        DiffCellKind::Delete => "bg-[var(--diff-word-del)]",
        _ => "",
    };
    view! {
        <span class=format!("min-w-0 overflow-hidden whitespace-pre px-2 {row_class}")>
            {parts.into_iter().enumerate().map(|(index, (text, marked))| {
                view! { <span data-part=index class=if marked { mark_class } else { "" }>{text}</span> }
            }).collect_view()}
        </span>
    }
}
