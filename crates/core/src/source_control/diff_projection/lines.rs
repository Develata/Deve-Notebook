//! Borrowed line indexing and UTF-16 range conversion.
//! plan_ref:
//!   - 05_diff_logic#typed-diff-projection-contract

use super::{DiffByteRange, DiffCellKind, DiffCellProjection, DiffProjectionError, DiffTextRange};
use similar::{ChangeTag, TextDiff};
use std::time::Instant;

#[derive(Clone, Copy, Debug)]
pub(crate) struct LineSpan<'a> {
    pub text: &'a str,
    pub range: DiffByteRange,
}

pub(crate) fn line_count(
    content: &str,
    cancelled: &dyn Fn() -> bool,
) -> Result<usize, DiffProjectionError> {
    if content.is_empty() {
        return Ok(0);
    }
    let mut count = 1usize;
    for (index, byte) in content.bytes().enumerate() {
        if index % 4096 == 0 {
            check_cancel(cancelled)?;
        }
        if byte == b'\n' {
            count = count.saturating_add(1);
        }
    }
    Ok(count)
}

pub(crate) fn line_spans(content: &str, capacity: usize) -> Vec<LineSpan<'_>> {
    let mut spans = Vec::with_capacity(capacity);
    let mut offset = 0usize;
    for segment in content.split_inclusive('\n') {
        let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
        let text = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        let end = offset + text.len();
        spans.push(LineSpan {
            text,
            range: DiffByteRange {
                start: offset as u32,
                end: end as u32,
            },
        });
        offset += segment.len();
    }
    if content.ends_with('\n') {
        spans.push(LineSpan {
            text: "",
            range: DiffByteRange {
                start: content.len() as u32,
                end: content.len() as u32,
            },
        });
    } else if content.is_empty() {
        spans.clear();
    }
    spans
}

pub(crate) fn cell(
    span: LineSpan<'_>,
    line_number: usize,
    kind: DiffCellKind,
    word_ranges: Vec<DiffTextRange>,
) -> DiffCellProjection {
    DiffCellProjection {
        line_number: Some((line_number + 1) as u32),
        byte_range: Some(span.range),
        word_ranges,
        kind,
    }
}

pub(crate) fn word_ranges(
    old: &str,
    new: &str,
    deadline: Instant,
    cancelled: &dyn Fn() -> bool,
) -> Result<(Vec<DiffTextRange>, Vec<DiffTextRange>), DiffProjectionError> {
    check_cancel(cancelled)?;
    let diff = TextDiff::configure()
        .deadline(deadline)
        .diff_words(old, new);
    check_deadline(deadline)?;
    let mut old_utf16 = 0u32;
    let mut new_utf16 = 0u32;
    let mut old_ranges = Vec::new();
    let mut new_ranges = Vec::new();
    for change in diff.iter_all_changes() {
        check_cancel(cancelled)?;
        let len = change.value().encode_utf16().count() as u32;
        match change.tag() {
            ChangeTag::Equal => {
                old_utf16 += len;
                new_utf16 += len;
            }
            ChangeTag::Delete => {
                if len > 0 {
                    old_ranges.push(DiffTextRange {
                        start: old_utf16,
                        end: old_utf16 + len,
                    });
                }
                old_utf16 += len;
            }
            ChangeTag::Insert => {
                if len > 0 {
                    new_ranges.push(DiffTextRange {
                        start: new_utf16,
                        end: new_utf16 + len,
                    });
                }
                new_utf16 += len;
            }
        }
    }
    Ok((old_ranges, new_ranges))
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
