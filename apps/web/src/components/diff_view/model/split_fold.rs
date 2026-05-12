//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 03_rendering#large-document-runtime
//!
use super::{LineKind, LineView};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum SplitRow {
    Pair { left: LineView, right: LineView },
    Fold { id: usize, hidden_count: usize },
}

impl SplitRow {
    pub fn key(&self) -> String {
        match self {
            SplitRow::Pair { left, right } => {
                format!("P:{}:{}", left.num.unwrap_or(0), right.num.unwrap_or(0))
            }
            SplitRow::Fold { id, hidden_count } => format!("SF:{}:{}", id, hidden_count),
        }
    }
}

pub fn build_folded_split_rows(
    left: &[LineView],
    right: &[LineView],
    context: usize,
    folding_enabled: bool,
    expanded_folds: &HashSet<usize>,
) -> Vec<SplitRow> {
    let n = left.len().min(right.len());
    if !folding_enabled || n == 0 {
        return (0..n)
            .map(|i| SplitRow::Pair {
                left: left[i].clone(),
                right: right[i].clone(),
            })
            .collect();
    }

    let mut keep = vec![false; n];
    for i in 0..n {
        let changed = matches!(left[i].kind, LineKind::Add | LineKind::Del)
            || matches!(right[i].kind, LineKind::Add | LineKind::Del);
        if changed {
            let start = i.saturating_sub(context);
            let end = (i + context + 1).min(n);
            for slot in keep.iter_mut().take(end).skip(start) {
                *slot = true;
            }
        }
    }

    let mut rows = Vec::with_capacity(n);
    let mut i = 0usize;
    while i < n {
        if keep[i] {
            rows.push(SplitRow::Pair {
                left: left[i].clone(),
                right: right[i].clone(),
            });
            i += 1;
            continue;
        }
        let start = i;
        while i < n && !keep[i] {
            i += 1;
        }
        let hidden_count = i - start;
        if hidden_count <= context.saturating_mul(2) || expanded_folds.contains(&start) {
            for j in start..i {
                rows.push(SplitRow::Pair {
                    left: left[j].clone(),
                    right: right[j].clone(),
                });
            }
        } else {
            rows.push(SplitRow::Fold {
                id: start,
                hidden_count,
            });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::{SplitRow, build_folded_split_rows};
    use crate::components::diff_view::model::{LineKind, LineView};
    use std::collections::HashSet;

    fn line(idx: usize, kind: LineKind) -> LineView {
        LineView {
            num: Some(idx + 1),
            content: format!("line-{idx}"),
            class: "",
            word_ranges: Vec::new(),
            kind,
        }
    }

    #[test]
    fn split_diff_fold_rows_respect_expanded_region() {
        let left: Vec<_> = (0..20)
            .map(|i| {
                line(
                    i,
                    if i == 10 {
                        LineKind::Del
                    } else {
                        LineKind::Normal
                    },
                )
            })
            .collect();
        let right: Vec<_> = (0..20).map(|i| line(i, LineKind::Normal)).collect();

        let folded = build_folded_split_rows(&left, &right, 3, true, &HashSet::new());
        assert!(
            folded
                .iter()
                .any(|row| matches!(row, SplitRow::Fold { id: 0, .. }))
        );

        let expanded = build_folded_split_rows(&left, &right, 3, true, &HashSet::from([0usize]));
        assert!(
            expanded
                .iter()
                .all(|row| matches!(row, SplitRow::Pair { .. }))
        );
    }
}
