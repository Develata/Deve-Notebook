// crates\core\src\ledger\merge\region.rs
//! plan_ref:
//!   - 04_storage#facts-partition
//!   - 03_rendering#document-authority-bridge
//!
// ---------------------------------------------------------------
// 模块：三路合并 region 聚合
// 作用：把 local/remote 编辑序列折叠为可应用编辑或冲突片段
// 功能：重叠 region 扩展、等价编辑检测、冲突 hunk 构建
// ---------------------------------------------------------------

use super::diff::{Edit, apply_edits, edits_equivalent, edits_overlap, slice_by_char};
use super::types::ConflictHunk;

pub(super) struct RegionMerge {
    pub(super) merged_edits: Vec<Edit>,
    pub(super) conflicts: Vec<ConflictHunk>,
}

pub(super) fn merge_regions(
    base: &str,
    local_edits: &[Edit],
    remote_edits: &[Edit],
) -> RegionMerge {
    let mut merged_edits = Vec::new();
    let mut conflicts = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;

    while i < local_edits.len() || j < remote_edits.len() {
        let region = next_region(base, local_edits, remote_edits, &mut i, &mut j);
        match (region.merge_edit, region.conflict) {
            (Some(edit), _) => merged_edits.push(edit),
            (None, Some(conflict)) => conflicts.push(conflict),
            (None, None) => {}
        }
    }

    RegionMerge {
        merged_edits,
        conflicts,
    }
}

struct MergeRegion {
    merge_edit: Option<Edit>,
    conflict: Option<ConflictHunk>,
}

fn next_region(
    base: &str,
    local_edits: &[Edit],
    remote_edits: &[Edit],
    i: &mut usize,
    j: &mut usize,
) -> MergeRegion {
    let seed = select_seed(local_edits.get(*i), remote_edits.get(*j));
    let (start, end) = (seed.start, seed.end);
    let mut local = collect_overlapping(local_edits, i, seed);
    let mut remote = collect_overlapping(remote_edits, j, seed);
    let mut span = span_for(&local, &remote, start, end);

    loop {
        let before = (local.len(), remote.len(), span.start, span.end);
        local.extend(collect_overlapping_span(local_edits, i, span));
        remote.extend(collect_overlapping_span(remote_edits, j, span));
        span = span_for(&local, &remote, span.start, span.end);
        if before == (local.len(), remote.len(), span.start, span.end) {
            break;
        }
    }

    if remote.is_empty() {
        return MergeRegion {
            merge_edit: coalesce_region(base, span, &local),
            conflict: None,
        };
    }
    if local.is_empty() {
        return MergeRegion {
            merge_edit: coalesce_region(base, span, &remote),
            conflict: None,
        };
    }
    if single_equivalent(&local, &remote) {
        return MergeRegion {
            merge_edit: Some(local[0].clone()),
            conflict: None,
        };
    }

    let local_replacement = replacement_for_region(base, span, &local);
    let remote_replacement = replacement_for_region(base, span, &remote);
    if local_replacement == remote_replacement {
        return MergeRegion {
            merge_edit: Some(Edit {
                start: span.start,
                end: span.end,
                replacement: local_replacement,
            }),
            conflict: None,
        };
    }

    MergeRegion {
        merge_edit: None,
        conflict: Some(build_region_conflict_hunk(
            base,
            span,
            local_replacement,
            remote_replacement,
        )),
    }
}

#[derive(Clone, Copy)]
struct Span {
    start: usize,
    end: usize,
}

fn select_seed<'a>(local: Option<&'a Edit>, remote: Option<&'a Edit>) -> &'a Edit {
    match (local, remote) {
        (Some(local), Some(remote)) if local.start <= remote.start => local,
        (Some(_), Some(remote)) => remote,
        (Some(local), None) => local,
        (None, Some(remote)) => remote,
        (None, None) => unreachable!("merge loop must have at least one pending edit"),
    }
}

fn collect_overlapping(edits: &[Edit], cursor: &mut usize, seed: &Edit) -> Vec<Edit> {
    let mut collected = Vec::new();
    while let Some(edit) = edits.get(*cursor) {
        if !edits_overlap(edit, seed) && edit.start != seed.start {
            break;
        }
        collected.push(edit.clone());
        *cursor += 1;
    }
    collected
}

fn collect_overlapping_span(edits: &[Edit], cursor: &mut usize, span: Span) -> Vec<Edit> {
    let mut collected = Vec::new();
    while let Some(edit) = edits.get(*cursor) {
        if !edit_overlaps_span(edit, span) {
            break;
        }
        collected.push(edit.clone());
        *cursor += 1;
    }
    collected
}

fn edit_overlaps_span(edit: &Edit, span: Span) -> bool {
    let region = Edit {
        start: span.start,
        end: span.end,
        replacement: String::new(),
    };
    edits_overlap(edit, &region)
        || (edit.start == edit.end && (edit.start == span.start || edit.start == span.end))
}

fn span_for(local: &[Edit], remote: &[Edit], start: usize, end: usize) -> Span {
    local
        .iter()
        .chain(remote.iter())
        .fold(Span { start, end }, |span, edit| Span {
            start: span.start.min(edit.start),
            end: span.end.max(edit.end),
        })
}

fn single_equivalent(local: &[Edit], remote: &[Edit]) -> bool {
    local.len() == 1 && remote.len() == 1 && edits_equivalent(&local[0], &remote[0])
}

fn coalesce_region(base: &str, span: Span, edits: &[Edit]) -> Option<Edit> {
    match edits {
        [] => None,
        [edit] => Some(edit.clone()),
        _ => Some(Edit {
            start: span.start,
            end: span.end,
            replacement: replacement_for_region(base, span, edits),
        }),
    }
}

fn replacement_for_region(base: &str, span: Span, edits: &[Edit]) -> String {
    let base_slice = slice_by_char(base, span.start, span.end);
    let relative_edits = edits
        .iter()
        .map(|edit| Edit {
            start: edit.start.saturating_sub(span.start),
            end: edit.end.saturating_sub(span.start),
            replacement: edit.replacement.clone(),
        })
        .collect::<Vec<_>>();
    apply_edits(base_slice, &relative_edits)
}

fn build_region_conflict_hunk(
    base: &str,
    span: Span,
    local_replacement: String,
    remote_replacement: String,
) -> ConflictHunk {
    let start_line = char_index_to_line(base, span.start);
    let end_line = char_index_to_line(base, span.end);
    let length = end_line.saturating_sub(start_line).saturating_add(1);

    ConflictHunk {
        start_line,
        length,
        local_lines: to_lines(&local_replacement),
        remote_lines: to_lines(&remote_replacement),
    }
}

fn char_index_to_line(s: &str, char_index: usize) -> usize {
    let mut line = 0usize;
    for (count, ch) in s.chars().enumerate() {
        if count >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
        }
    }
    line
}

fn to_lines(s: &str) -> Vec<String> {
    if s.is_empty() {
        return Vec::new();
    }
    s.lines().map(|line| line.to_string()).collect()
}
