use super::{LineKind, LineView};
use similar::{ChangeTag, TextDiff};

type Ranges = Vec<(usize, usize)>;

fn word_ranges(old: &str, new: &str) -> (Ranges, Ranges) {
    let diff = TextDiff::from_words(old, new);
    let mut old_pos = 0usize;
    let mut new_pos = 0usize;
    let mut old_ranges = Vec::new();
    let mut new_ranges = Vec::new();
    for c in diff.iter_all_changes() {
        let len = c.value().len();
        match c.tag() {
            ChangeTag::Equal => {
                old_pos += len;
                new_pos += len;
            }
            ChangeTag::Delete => {
                if len > 0 {
                    old_ranges.push((old_pos, old_pos + len));
                }
                old_pos += len;
            }
            ChangeTag::Insert => {
                if len > 0 {
                    new_ranges.push((new_pos, new_pos + len));
                }
                new_pos += len;
            }
        }
    }
    (old_ranges, new_ranges)
}

pub(super) struct ReplaceCtx<'a> {
    pub old_lines: &'a [&'a str],
    pub new_lines: &'a [&'a str],
    pub old_offset: usize,
    pub new_offset: usize,
}

pub(super) fn append_replace_lines(
    ctx: ReplaceCtx<'_>,
    old_idx: &[usize],
    new_idx: &[usize],
    left: &mut Vec<LineView>,
    right: &mut Vec<LineView>,
) -> isize {
    let mut delta = 0isize;
    let pair_count = old_idx.len().min(new_idx.len());
    for p in 0..pair_count {
        let oi = old_idx[p];
        let nj = new_idx[p];
        let old_text = ctx
            .old_lines
            .get(oi)
            .copied()
            .unwrap_or_default()
            .to_string();
        let new_text = ctx
            .new_lines
            .get(nj)
            .copied()
            .unwrap_or_default()
            .to_string();
        let (old_ranges, new_ranges) = word_ranges(&old_text, &new_text);
        left.push(LineView {
            num: Some(ctx.old_offset + oi + 1),
            content: old_text,
            class: "bg-[var(--diff-line-del)]",
            word_ranges: old_ranges,
            kind: LineKind::Del,
        });
        right.push(LineView {
            num: Some(ctx.new_offset + nj + 1),
            content: new_text,
            class: "bg-[var(--diff-line-add)]",
            word_ranges: new_ranges,
            kind: LineKind::Add,
        });
    }

    for oi in old_idx.iter().skip(pair_count).copied() {
        left.push(LineView {
            num: Some(ctx.old_offset + oi + 1),
            content: ctx
                .old_lines
                .get(oi)
                .copied()
                .unwrap_or_default()
                .to_string(),
            class: "bg-[var(--diff-line-del)]",
            word_ranges: Vec::new(),
            kind: LineKind::Del,
        });
        right.push(LineView::empty());
        delta -= 1;
    }
    for nj in new_idx.iter().skip(pair_count).copied() {
        left.push(LineView::empty());
        right.push(LineView {
            num: Some(ctx.new_offset + nj + 1),
            content: ctx
                .new_lines
                .get(nj)
                .copied()
                .unwrap_or_default()
                .to_string(),
            class: "bg-[var(--diff-line-add)]",
            word_ranges: Vec::new(),
            kind: LineKind::Add,
        });
        delta += 1;
    }
    delta
}
