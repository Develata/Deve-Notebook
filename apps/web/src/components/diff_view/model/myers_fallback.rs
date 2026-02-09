use super::replace_word::{ReplaceCtx, append_replace_lines};
use super::{LineKind, LineView};
use similar::{DiffTag, TextDiff};

/// 对指定行切片执行 Myers 行级 diff，并写入渲染行。
///
/// Post-conditions:
/// - `left.len() == right.len()`。
/// - 返回值为 `new_len - old_len` 的净增量。
pub fn apply_lines_diff(
    old_lines: &[&str],
    new_lines: &[&str],
    old_offset: usize,
    new_offset: usize,
    left: &mut Vec<LineView>,
    right: &mut Vec<LineView>,
) -> isize {
    let old_text = old_lines.join("\n");
    let new_text = new_lines.join("\n");
    let diff = TextDiff::from_lines(&old_text, &new_text);
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
                    left,
                    right,
                );
            }
        }
    }

    delta
}
