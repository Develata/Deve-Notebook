//! Whole-document Patience + Myers row construction.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract

use std::collections::HashMap;
use std::time::Instant;

use similar::{Algorithm, DiffOp, capture_diff_slices_deadline};

use super::error::DiffProjectionError;
use super::lines::{LineSpan, cell, word_ranges};
use super::{DiffAlgorithm, DiffCellKind, DiffCellProjection, DiffRowProjection};

pub(crate) struct RowBuild {
    pub rows: Vec<DiffRowProjection>,
    pub algorithm: DiffAlgorithm,
    pub added: u32,
    pub deleted: u32,
}

pub(crate) fn build_rows(
    old: &[LineSpan<'_>],
    new: &[LineSpan<'_>],
    deadline: Instant,
    cancelled: &dyn Fn() -> bool,
) -> Result<RowBuild, DiffProjectionError> {
    check_cancel(cancelled)?;
    let old_text: Vec<&str> = old.iter().map(|line| line.text).collect();
    let new_text: Vec<&str> = new.iter().map(|line| line.text).collect();
    let anchors = patience_anchors(&old_text, &new_text, cancelled)?;
    let mut builder = Builder::new(old, new, &old_text, &new_text, deadline, cancelled);
    let mut old_start = 0usize;
    let mut new_start = 0usize;
    for (old_anchor, new_anchor) in &anchors {
        builder.append_myers(old_start..*old_anchor, new_start..*new_anchor)?;
        builder.push_pair(*old_anchor, *new_anchor, DiffCellKind::Context, false)?;
        old_start = *old_anchor + 1;
        new_start = *new_anchor + 1;
    }
    builder.append_myers(old_start..old.len(), new_start..new.len())?;
    Ok(RowBuild {
        rows: builder.rows,
        algorithm: if anchors.is_empty() {
            DiffAlgorithm::Myers
        } else {
            DiffAlgorithm::PatienceMyers
        },
        added: builder.added,
        deleted: builder.deleted,
    })
}

struct Builder<'a> {
    old: &'a [LineSpan<'a>],
    new: &'a [LineSpan<'a>],
    old_text: &'a [&'a str],
    new_text: &'a [&'a str],
    rows: Vec<DiffRowProjection>,
    added: u32,
    deleted: u32,
    deadline: Instant,
    cancelled: &'a dyn Fn() -> bool,
}

impl<'a> Builder<'a> {
    fn new(
        old: &'a [LineSpan<'a>],
        new: &'a [LineSpan<'a>],
        old_text: &'a [&'a str],
        new_text: &'a [&'a str],
        deadline: Instant,
        cancelled: &'a dyn Fn() -> bool,
    ) -> Self {
        Self {
            old,
            new,
            old_text,
            new_text,
            rows: Vec::with_capacity(old.len().max(new.len())),
            added: 0,
            deleted: 0,
            deadline,
            cancelled,
        }
    }

    fn append_myers(
        &mut self,
        old_range: std::ops::Range<usize>,
        new_range: std::ops::Range<usize>,
    ) -> Result<(), DiffProjectionError> {
        check_cancel(self.cancelled)?;
        let ops = capture_diff_slices_deadline(
            Algorithm::Myers,
            &self.old_text[old_range.clone()],
            &self.new_text[new_range.clone()],
            Some(self.deadline),
        );
        check_deadline(self.deadline)?;
        for op in ops {
            check_cancel(self.cancelled)?;
            self.append_op(&op, old_range.start, new_range.start)?;
        }
        Ok(())
    }

    fn append_op(
        &mut self,
        op: &DiffOp,
        old_offset: usize,
        new_offset: usize,
    ) -> Result<(), DiffProjectionError> {
        let old = op.old_range();
        let new = op.new_range();
        match op.tag() {
            similar::DiffTag::Equal => {
                for (left, right) in old.zip(new) {
                    check_cancel(self.cancelled)?;
                    self.push_pair(
                        old_offset + left,
                        new_offset + right,
                        DiffCellKind::Context,
                        false,
                    )?;
                }
            }
            similar::DiffTag::Delete => {
                for left in old {
                    check_cancel(self.cancelled)?;
                    self.push_left(old_offset + left);
                }
            }
            similar::DiffTag::Insert => {
                for right in new {
                    check_cancel(self.cancelled)?;
                    self.push_right(new_offset + right);
                }
            }
            similar::DiffTag::Replace => {
                let old_idx: Vec<_> = old.collect();
                let new_idx: Vec<_> = new.collect();
                let paired = old_idx.len().min(new_idx.len());
                for index in 0..paired {
                    check_cancel(self.cancelled)?;
                    self.push_pair(
                        old_offset + old_idx[index],
                        new_offset + new_idx[index],
                        DiffCellKind::Delete,
                        true,
                    )?;
                }
                for left in old_idx.into_iter().skip(paired) {
                    check_cancel(self.cancelled)?;
                    self.push_left(old_offset + left);
                }
                for right in new_idx.into_iter().skip(paired) {
                    check_cancel(self.cancelled)?;
                    self.push_right(new_offset + right);
                }
            }
        }
        Ok(())
    }

    fn push_pair(
        &mut self,
        left: usize,
        right: usize,
        kind: DiffCellKind,
        replace: bool,
    ) -> Result<(), DiffProjectionError> {
        let (left_ranges, right_ranges) = if replace {
            word_ranges(
                self.old[left].text,
                self.new[right].text,
                self.deadline,
                self.cancelled,
            )?
        } else {
            (Vec::new(), Vec::new())
        };
        let right_kind = if replace { DiffCellKind::Add } else { kind };
        self.push_row(
            cell(self.old[left], left, kind, left_ranges),
            cell(self.new[right], right, right_kind, right_ranges),
        );
        if replace {
            self.deleted += 1;
            self.added += 1;
        }
        Ok(())
    }

    fn push_left(&mut self, left: usize) {
        self.deleted += 1;
        self.push_row(
            cell(self.old[left], left, DiffCellKind::Delete, Vec::new()),
            DiffCellProjection::empty(),
        );
    }

    fn push_right(&mut self, right: usize) {
        self.added += 1;
        self.push_row(
            DiffCellProjection::empty(),
            cell(self.new[right], right, DiffCellKind::Add, Vec::new()),
        );
    }

    fn push_row(&mut self, left: DiffCellProjection, right: DiffCellProjection) {
        self.rows.push(DiffRowProjection {
            row_id: self.rows.len() as u32,
            left,
            right,
            hunk_id: None,
        });
    }
}

fn patience_anchors(
    old: &[&str],
    new: &[&str],
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<(usize, usize)>, DiffProjectionError> {
    let mut old_count = HashMap::new();
    let mut new_count = HashMap::new();
    for line in old {
        check_cancel(cancelled)?;
        *old_count.entry(*line).or_insert(0usize) += 1;
    }
    for line in new {
        check_cancel(cancelled)?;
        *new_count.entry(*line).or_insert(0usize) += 1;
    }
    let mut new_unique = HashMap::new();
    for (index, line) in new.iter().enumerate() {
        check_cancel(cancelled)?;
        if new_count.get(line) == Some(&1) {
            new_unique.insert(*line, index);
        }
    }
    let mut candidates = Vec::new();
    for (index, line) in old.iter().enumerate() {
        check_cancel(cancelled)?;
        if old_count.get(line) == Some(&1)
            && let Some(new_index) = new_unique.get(line)
        {
            candidates.push((index, *new_index));
        }
    }
    longest_increasing(&candidates, cancelled)
}

fn longest_increasing(
    candidates: &[(usize, usize)],
    cancelled: &dyn Fn() -> bool,
) -> Result<Vec<(usize, usize)>, DiffProjectionError> {
    let mut tails = Vec::<usize>::new();
    let mut previous = vec![None; candidates.len()];
    for index in 0..candidates.len() {
        check_cancel(cancelled)?;
        let key = candidates[index].1;
        let position = tails
            .binary_search_by(|tail| candidates[*tail].1.cmp(&key))
            .unwrap_or_else(|position| position);
        if position > 0 {
            previous[index] = Some(tails[position - 1]);
        }
        if position == tails.len() {
            tails.push(index);
        } else {
            tails[position] = index;
        }
    }
    let mut result = Vec::new();
    let mut current = tails.last().copied();
    while let Some(index) = current {
        check_cancel(cancelled)?;
        result.push(candidates[index]);
        current = previous[index];
    }
    result.reverse();
    Ok(result)
}

fn check_cancel(cancelled: &dyn Fn() -> bool) -> Result<(), DiffProjectionError> {
    if cancelled() {
        Err(DiffProjectionError::Cancelled)
    } else {
        Ok(())
    }
}

fn check_deadline(deadline: Instant) -> Result<(), DiffProjectionError> {
    if Instant::now() >= deadline {
        Err(DiffProjectionError::ComputeDeadline)
    } else {
        Ok(())
    }
}
