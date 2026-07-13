//! Hunk and context fold derivation from canonical rows.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract

use sha2::{Digest, Sha256};

use super::{
    DiffCellKind, DiffFoldRange, DiffHunkProjection, DiffLineRange, DiffProjectionError,
    DiffRowProjection,
};

pub(crate) fn attach_hunks(
    rows: &mut [DiffRowProjection],
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<DiffHunkProjection>, DiffProjectionError> {
    let mut hunks = Vec::new();
    let mut index = 0usize;
    while index < rows.len() {
        check_cancel(cancelled)?;
        if !changed(&rows[index]) {
            index += 1;
            continue;
        }
        let start = index;
        while index < rows.len() && changed(&rows[index]) {
            index += 1;
        }
        let hunk_id = hunks.len() as u32;
        for row in &mut rows[start..index] {
            check_cancel(cancelled)?;
            row.hunk_id = Some(hunk_id);
        }
        hunks.push(DiffHunkProjection {
            hunk_id,
            row_start: start as u32,
            row_end: index as u32,
            old_lines: line_range(rows, start, index, true),
            new_lines: line_range(rows, start, index, false),
        });
    }
    Ok(hunks)
}

pub(crate) fn build_folds(
    rows: &[DiffRowProjection],
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<DiffFoldRange>, DiffProjectionError> {
    let mut folds = Vec::new();
    for context in [3u8, 5, 8] {
        let mut keep = vec![false; rows.len()];
        for (index, row) in rows.iter().enumerate() {
            check_cancel(cancelled)?;
            if changed(row) {
                let start = index.saturating_sub(context as usize);
                let end = (index + context as usize + 1).min(rows.len());
                keep[start..end].fill(true);
            }
        }
        let mut index = 0usize;
        while index < rows.len() {
            check_cancel(cancelled)?;
            if keep[index] {
                index += 1;
                continue;
            }
            let start = index;
            while index < rows.len() && !keep[index] {
                index += 1;
            }
            if index - start > (context as usize).saturating_mul(2) {
                folds.push(DiffFoldRange {
                    fold_id: fold_id(start, index, context),
                    row_start: start as u32,
                    row_end: index as u32,
                    context_lines: context,
                });
            }
        }
    }
    Ok(folds)
}

fn changed(row: &DiffRowProjection) -> bool {
    !matches!(row.left.kind, DiffCellKind::Context | DiffCellKind::Empty)
        || !matches!(row.right.kind, DiffCellKind::Context | DiffCellKind::Empty)
}

fn line_range(
    all_rows: &[DiffRowProjection],
    start: usize,
    end: usize,
    left: bool,
) -> DiffLineRange {
    let mut first = None;
    let mut last = None;
    for number in all_rows[start..end].iter().filter_map(|row| {
        if left {
            row.left.line_number
        } else {
            row.right.line_number
        }
    }) {
        first.get_or_insert(number);
        last = Some(number);
    }
    match (first, last) {
        (Some(start), Some(end)) => DiffLineRange {
            start,
            end: end.saturating_add(1),
        },
        _ => {
            let insertion = all_rows[..start]
                .iter()
                .rev()
                .find_map(|row| side_line(row, left))
                .map(|line| line.saturating_add(1))
                .or_else(|| all_rows[end..].iter().find_map(|row| side_line(row, left)))
                .unwrap_or(1);
            DiffLineRange {
                start: insertion,
                end: insertion,
            }
        }
    }
}

fn side_line(row: &DiffRowProjection, left: bool) -> Option<u32> {
    if left {
        row.left.line_number
    } else {
        row.right.line_number
    }
}

fn fold_id(start: usize, end: usize, context: u8) -> String {
    let digest = Sha256::digest(format!("{start}:{end}:{context}").as_bytes());
    hex::encode(&digest[..8])
}

fn check_cancel(cancelled: &dyn Fn() -> bool) -> Result<(), DiffProjectionError> {
    if cancelled() {
        Err(DiffProjectionError::Cancelled)
    } else {
        Ok(())
    }
}
