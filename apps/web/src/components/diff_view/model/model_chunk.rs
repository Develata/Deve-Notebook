use super::replace_word::{ReplaceCtx, append_replace_lines};
use super::{LineKind, LineView};
use similar::{DiffTag, TextDiff};

fn compute_segment(
    old_lines: &[&str],
    new_lines: &[&str],
    old_offset: usize,
    new_offset: usize,
) -> (Vec<LineView>, Vec<LineView>, isize) {
    let old_text = old_lines.join("\n");
    let new_text = new_lines.join("\n");
    let diff = TextDiff::from_lines(&old_text, &new_text);
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut delta = 0isize;

    for op in diff.ops() {
        match op.tag() {
            DiffTag::Equal => {
                for (i, j) in op.old_range().zip(op.new_range()) {
                    left.push(LineView {
                        num: Some(old_offset + i + 1),
                        content: old_lines.get(i).copied().unwrap_or_default().to_string(),
                        class: "",
                        word_ranges: Vec::new(),
                        kind: LineKind::Normal,
                    });
                    right.push(LineView {
                        num: Some(new_offset + j + 1),
                        content: new_lines.get(j).copied().unwrap_or_default().to_string(),
                        class: "",
                        word_ranges: Vec::new(),
                        kind: LineKind::Normal,
                    });
                }
            }
            DiffTag::Delete => {
                for i in op.old_range() {
                    left.push(LineView {
                        num: Some(old_offset + i + 1),
                        content: old_lines.get(i).copied().unwrap_or_default().to_string(),
                        class: "bg-[var(--diff-line-del)]",
                        word_ranges: Vec::new(),
                        kind: LineKind::Del,
                    });
                    right.push(LineView::empty());
                    delta -= 1;
                }
            }
            DiffTag::Insert => {
                for j in op.new_range() {
                    left.push(LineView::empty());
                    right.push(LineView {
                        num: Some(new_offset + j + 1),
                        content: new_lines.get(j).copied().unwrap_or_default().to_string(),
                        class: "bg-[var(--diff-line-add)]",
                        word_ranges: Vec::new(),
                        kind: LineKind::Add,
                    });
                    delta += 1;
                }
            }
            DiffTag::Replace => {
                let old_idx: Vec<usize> = op.old_range().collect();
                let new_idx: Vec<usize> = op.new_range().collect();
                delta += append_replace_lines(
                    ReplaceCtx {
                        old_lines,
                        new_lines,
                        old_offset,
                        new_offset,
                    },
                    &old_idx,
                    &new_idx,
                    &mut left,
                    &mut right,
                );
            }
        }
    }

    (left, right, delta)
}

pub fn compute_diff_chunked_inner(
    old_content: &str,
    new_content: &str,
    chunk_size: usize,
) -> (Vec<LineView>, Vec<LineView>) {
    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut old_start = 0usize;
    let mut drift = 0isize;

    while old_start < old_lines.len() || ((old_start as isize) + drift) < (new_lines.len() as isize)
    {
        let old_end = (old_start + chunk_size).min(old_lines.len());
        let new_start = (old_start as isize + drift).max(0) as usize;
        let new_end = (new_start + chunk_size).min(new_lines.len());

        let (l, r, delta) = compute_segment(
            &old_lines[old_start..old_end],
            &new_lines[new_start..new_end],
            old_start,
            new_start,
        );
        left.extend(l);
        right.extend(r);

        old_start = old_end;
        drift += delta;

        if old_start >= old_lines.len() && new_end >= new_lines.len() {
            break;
        }
    }

    (left, right)
}
