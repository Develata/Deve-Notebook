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
