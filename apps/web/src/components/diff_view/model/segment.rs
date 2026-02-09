use super::myers_fallback::apply_lines_diff;
use super::patience::anchors;
use super::{LineKind, LineView};

struct SegmentCtx<'a> {
    old_lines: &'a [&'a str],
    new_lines: &'a [&'a str],
    old_offset: usize,
    new_offset: usize,
}

fn push_anchor_equal(
    ctx: &SegmentCtx<'_>,
    oi: usize,
    nj: usize,
    left: &mut Vec<LineView>,
    right: &mut Vec<LineView>,
) {
    left.push(LineView {
        num: Some(ctx.old_offset + oi + 1),
        content: ctx
            .old_lines
            .get(oi)
            .copied()
            .unwrap_or_default()
            .to_string(),
        class: "",
        word_ranges: Vec::new(),
        kind: LineKind::Normal,
    });
    right.push(LineView {
        num: Some(ctx.new_offset + nj + 1),
        content: ctx
            .new_lines
            .get(nj)
            .copied()
            .unwrap_or_default()
            .to_string(),
        class: "",
        word_ranges: Vec::new(),
        kind: LineKind::Normal,
    });
}

/// 分段计算 Diff，优先 Patience 锚点切分，再应用 Myers。
pub fn compute_segment(
    old_lines: &[&str],
    new_lines: &[&str],
    old_offset: usize,
    new_offset: usize,
) -> (Vec<LineView>, Vec<LineView>, isize, bool) {
    let ctx = SegmentCtx {
        old_lines,
        new_lines,
        old_offset,
        new_offset,
    };
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut delta = 0isize;
    let anchors = anchors(old_lines, new_lines);

    if anchors.is_empty() {
        delta += apply_lines_diff(
            ctx.old_lines,
            ctx.new_lines,
            ctx.old_offset,
            ctx.new_offset,
            &mut left,
            &mut right,
        );
        return (left, right, delta, false);
    }

    let mut prev_old = 0usize;
    let mut prev_new = 0usize;
    for (oi, nj) in anchors {
        delta += apply_lines_diff(
            &old_lines[prev_old..oi],
            &new_lines[prev_new..nj],
            ctx.old_offset + prev_old,
            ctx.new_offset + prev_new,
            &mut left,
            &mut right,
        );
        push_anchor_equal(&ctx, oi, nj, &mut left, &mut right);
        prev_old = oi + 1;
        prev_new = nj + 1;
    }

    delta += apply_lines_diff(
        &old_lines[prev_old..],
        &new_lines[prev_new..],
        ctx.old_offset + prev_old,
        ctx.new_offset + prev_new,
        &mut left,
        &mut right,
    );

    (left, right, delta, true)
}
