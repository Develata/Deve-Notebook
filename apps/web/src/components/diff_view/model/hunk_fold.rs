//! plan_ref:
//!   - 05_diff_logic#source-control-runtime
//!   - 10_rendering#large-document-runtime
//!
use super::{LineKind, UnifiedLine};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub enum UnifiedRow {
    Line(UnifiedLine),
    Fold { id: usize, hidden_count: usize },
}

impl UnifiedRow {
    pub fn key(&self) -> String {
        match self {
            UnifiedRow::Line(line) => format!("L:{}:{}", line.num.unwrap_or(0), line.content),
            UnifiedRow::Fold { id, hidden_count } => format!("F:{}:{}", id, hidden_count),
        }
    }
}

pub fn build_folded_rows(
    lines: &[UnifiedLine],
    context: usize,
    folding_enabled: bool,
    expanded_folds: &HashSet<usize>,
) -> Vec<UnifiedRow> {
    if !folding_enabled || lines.is_empty() {
        return lines.iter().cloned().map(UnifiedRow::Line).collect();
    }

    let n = lines.len();
    let mut keep = vec![false; n];
    for (idx, line) in lines.iter().enumerate() {
        if matches!(line.kind, LineKind::Add | LineKind::Del) {
            let start = idx.saturating_sub(context);
            let end = (idx + context + 1).min(n);
            for slot in keep.iter_mut().take(end).skip(start) {
                *slot = true;
            }
        }
    }

    let mut rows = Vec::with_capacity(n);
    let mut i = 0usize;
    while i < n {
        if keep[i] {
            rows.push(UnifiedRow::Line(lines[i].clone()));
            i += 1;
            continue;
        }

        let start = i;
        while i < n && !keep[i] {
            i += 1;
        }
        let hidden_count = i - start;

        if hidden_count <= context.saturating_mul(2) || expanded_folds.contains(&start) {
            for line in lines.iter().take(i).skip(start) {
                rows.push(UnifiedRow::Line(line.clone()));
            }
        } else {
            rows.push(UnifiedRow::Fold {
                id: start,
                hidden_count,
            });
        }
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::{UnifiedRow, build_folded_rows};
    use crate::components::diff_view::model::{LineKind, UnifiedLine};
    use std::collections::HashSet;

    fn line(idx: usize, kind: LineKind) -> UnifiedLine {
        UnifiedLine {
            num: Some(idx + 1),
            content: format!("line-{idx}"),
            class: "",
            word_ranges: Vec::new(),
            kind,
        }
    }

    #[test]
    fn diff_fold_rows_collapse_and_expand_unchanged_region() {
        let mut lines: Vec<_> = (0..20).map(|i| line(i, LineKind::Normal)).collect();
        lines[10] = line(10, LineKind::Add);

        let folded = build_folded_rows(&lines, 3, true, &HashSet::new());
        assert!(
            folded
                .iter()
                .any(|row| matches!(row, UnifiedRow::Fold { id: 0, .. }))
        );

        let expanded = build_folded_rows(&lines, 3, true, &HashSet::from([0usize]));
        assert!(
            expanded
                .iter()
                .all(|row| matches!(row, UnifiedRow::Line(_)))
        );
    }

    #[test]
    fn diff_context_lines_change_fold_count() {
        let mut lines: Vec<_> = (0..30).map(|i| line(i, LineKind::Normal)).collect();
        lines[15] = line(15, LineKind::Del);

        let context_3 = build_folded_rows(&lines, 3, true, &HashSet::new());
        let context_8 = build_folded_rows(&lines, 8, true, &HashSet::new());
        let visible_3 = context_3
            .iter()
            .filter(|row| matches!(row, UnifiedRow::Line(_)))
            .count();
        let visible_8 = context_8
            .iter()
            .filter(|row| matches!(row, UnifiedRow::Line(_)))
            .count();

        assert!(visible_8 > visible_3);
    }
}
