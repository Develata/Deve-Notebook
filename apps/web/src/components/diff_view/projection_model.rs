//! Immutable display selection derived from a backend diff projection.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract
//!   - 10_rendering#large-document-runtime

use std::collections::{BTreeMap, BTreeSet, HashSet};

use super::projection_text::validate_highlight_ranges;
use deve_core::source_control::diff_projection::{
    DiffCellKind, DiffCellProjection, DiffProjection,
};

pub(super) const LINE_HEIGHT_PX: usize = 20;
const VIEWPORT_CHUNK: usize = 80;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Side {
    Left,
    Right,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum DisplayLine {
    Fold {
        id: String,
        hidden: u32,
        row_start: u32,
    },
    Split {
        row: u32,
    },
    Unified {
        row: u32,
        side: Side,
    },
}

impl DisplayLine {
    pub(super) fn key(&self) -> String {
        match self {
            Self::Fold { id, .. } => format!("fold:{id}"),
            Self::Split { row } => format!("split:{row}"),
            Self::Unified { row, side } => format!("unified:{row}:{side:?}"),
        }
    }

    fn row_start(&self) -> u32 {
        match self {
            Self::Fold { row_start, .. } => *row_start,
            Self::Split { row } | Self::Unified { row, .. } => *row,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct VirtualWindow {
    pub(super) start: usize,
    pub(super) end: usize,
}

impl VirtualWindow {
    pub(super) fn for_viewport(total: usize, scroll_top: usize, viewport_height: usize) -> Self {
        if total == 0 {
            return Self { start: 0, end: 0 };
        }
        let visible_start = scroll_top / LINE_HEIGHT_PX;
        let visible_count = viewport_height.div_ceil(LINE_HEIGHT_PX).max(1);
        Self {
            start: visible_start.saturating_sub(VIEWPORT_CHUNK).min(total),
            end: visible_start
                .saturating_add(visible_count)
                .saturating_add(VIEWPORT_CHUNK)
                .min(total),
        }
    }
}

pub(super) fn display_lines(
    projection: &DiffProjection,
    unified: bool,
    context_lines: u8,
    folding: bool,
    expanded: &BTreeSet<String>,
) -> Vec<DisplayLine> {
    let mut folds = BTreeMap::new();
    if folding {
        for fold in projection
            .folds
            .iter()
            .filter(|fold| fold.context_lines == context_lines && !expanded.contains(&fold.fold_id))
        {
            folds.insert(fold.row_start, fold);
        }
    }
    let mut canonical = Vec::with_capacity(projection.rows.len());
    let mut row = 0u32;
    while (row as usize) < projection.rows.len() {
        if let Some(fold) = folds.get(&row) {
            canonical.push(DisplayLine::Fold {
                id: fold.fold_id.clone(),
                hidden: fold.row_end.saturating_sub(fold.row_start),
                row_start: fold.row_start,
            });
            row = fold.row_end;
        } else {
            canonical.push(DisplayLine::Split { row });
            row += 1;
        }
    }
    if !unified {
        return canonical;
    }
    let mut result = Vec::with_capacity(canonical.len());
    for item in canonical {
        match item {
            DisplayLine::Fold { .. } => result.push(item),
            DisplayLine::Split { row } => {
                let projected = &projection.rows[row as usize];
                if projected.left.kind == DiffCellKind::Delete {
                    result.push(DisplayLine::Unified {
                        row,
                        side: Side::Left,
                    });
                }
                if projected.right.kind != DiffCellKind::Empty {
                    result.push(DisplayLine::Unified {
                        row,
                        side: Side::Right,
                    });
                }
            }
            DisplayLine::Unified { .. } => unreachable!(),
        }
    }
    result
}

pub(super) fn display_index_for_row(lines: &[DisplayLine], row_start: u32) -> usize {
    lines
        .iter()
        .position(|line| line.row_start() >= row_start)
        .unwrap_or_else(|| lines.len().saturating_sub(1))
}

pub(super) fn validate_projection(projection: &DiffProjection) -> Result<(), &'static str> {
    let row_count = projection.rows.len() as u32;
    let hunk_ids: HashSet<u32> = projection.hunks.iter().map(|hunk| hunk.hunk_id).collect();
    for (index, row) in projection.rows.iter().enumerate() {
        if row.row_id as usize != index {
            return Err("non-canonical row id");
        }
        validate_cell(projection, &row.left, true)?;
        validate_cell(projection, &row.right, false)?;
        if row.hunk_id.is_some_and(|id| !hunk_ids.contains(&id)) {
            return Err("unknown hunk id");
        }
    }
    for hunk in &projection.hunks {
        if hunk.row_start >= hunk.row_end || hunk.row_end > row_count {
            return Err("invalid hunk range");
        }
    }
    for fold in &projection.folds {
        if fold.row_start >= fold.row_end
            || fold.row_end > row_count
            || !matches!(fold.context_lines, 3 | 5 | 8)
        {
            return Err("invalid fold range");
        }
    }
    Ok(())
}

fn validate_cell(
    projection: &DiffProjection,
    cell: &DiffCellProjection,
    left: bool,
) -> Result<(), &'static str> {
    match (cell.kind, cell.byte_range) {
        (DiffCellKind::Empty, None) => return Ok(()),
        (DiffCellKind::Empty, Some(_)) | (_, None) => return Err("invalid empty cell"),
        _ => {}
    }
    let text = projection
        .cell_text(cell, left)
        .ok_or("invalid cell byte range")?;
    validate_highlight_ranges(text, &cell.word_ranges)
}

#[cfg(test)]
mod tests {
    use super::{
        DisplayLine, VirtualWindow, display_index_for_row, display_lines, validate_projection,
    };
    use deve_core::source_control::diff_projection::compute_diff_projection;
    use std::collections::BTreeSet;

    #[test]
    fn split_and_unified_only_select_backend_rows() {
        let projection = compute_diff_projection("a\nold\nz".into(), "a\nnew\nz".into()).unwrap();
        validate_projection(&projection).unwrap();
        let split = display_lines(&projection, false, 5, false, &BTreeSet::new());
        let unified = display_lines(&projection, true, 5, false, &BTreeSet::new());
        assert_eq!(split.len(), projection.rows.len());
        assert!(unified.len() >= split.len());
        assert!(
            unified
                .iter()
                .all(|line| matches!(line, DisplayLine::Unified { .. }))
        );
    }

    #[test]
    fn viewport_keeps_bounded_window() {
        let window = VirtualWindow::for_viewport(3_000, 0, 600);
        assert_eq!(window.start, 0);
        assert!(window.end <= 110);
    }

    #[test]
    fn hunk_navigation_targets_virtualized_row_index() {
        let lines = vec![
            DisplayLine::Split { row: 0 },
            DisplayLine::Fold {
                id: "f".into(),
                hidden: 50,
                row_start: 1,
            },
            DisplayLine::Split { row: 51 },
        ];
        assert_eq!(display_index_for_row(&lines, 51), 2);
    }
}
