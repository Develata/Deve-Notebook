// crates\core\src\ledger\merge\diff.rs
// ---------------------------------------------------------------
// 模块：差异编辑工具
// 作用：把 dissimilar diff 转换为可合并的编辑序列
// 功能：生成编辑、检测重叠、应用编辑到基准文本
// ---------------------------------------------------------------

use dissimilar::Chunk;

/// 基于字符索引的编辑操作
#[derive(Debug, Clone)]
pub(crate) struct Edit {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) replacement: String,
}

/// 将 base->other 的差异转换为编辑序列
///
/// 复杂点：dissimilar 输出的 Insert/Delete 需要合并成替换区间
pub(crate) fn diff_to_edits(base: &str, other: &str) -> Vec<Edit> {
    let chunks = dissimilar::diff(base, other);
    let mut edits: Vec<Edit> = Vec::new();
    let mut base_pos = 0usize;
    let mut pending_delete: Option<(usize, usize)> = None;

    for chunk in chunks {
        match chunk {
            Chunk::Equal(text) => {
                if let Some((start, len)) = pending_delete.take() {
                    push_edit(
                        &mut edits,
                        Edit {
                            start,
                            end: start + len,
                            replacement: String::new(),
                        },
                    );
                }
                base_pos += text.chars().count();
            }
            Chunk::Delete(text) => {
                let len = text.chars().count();
                pending_delete = match pending_delete {
                    Some((start, prev_len)) => Some((start, prev_len + len)),
                    None => Some((base_pos, len)),
                };
                base_pos += len;
            }
            Chunk::Insert(text) => {
                if let Some((start, len)) = pending_delete.take() {
                    push_edit(
                        &mut edits,
                        Edit {
                            start,
                            end: start + len,
                            replacement: text.to_string(),
                        },
                    );
                } else {
                    push_edit(
                        &mut edits,
                        Edit {
                            start: base_pos,
                            end: base_pos,
                            replacement: text.to_string(),
                        },
                    );
                }
            }
        }
    }

    if let Some((start, len)) = pending_delete.take() {
        push_edit(
            &mut edits,
            Edit {
                start,
                end: start + len,
                replacement: String::new(),
            },
        );
    }

    edits
}

/// 应用编辑到基准文本，返回合并结果
pub(crate) fn apply_edits(base: &str, edits: &[Edit]) -> String {
    let mut output = String::new();
    let mut cursor = 0usize;

    for edit in edits {
        if cursor < edit.start {
            output.push_str(slice_by_char(base, cursor, edit.start));
        }
        output.push_str(&edit.replacement);
        cursor = edit.end;
    }

    let base_len = base.chars().count();
    if cursor < base_len {
        output.push_str(slice_by_char(base, cursor, base_len));
    }

    output
}

/// 判断两个编辑是否完全等价
pub(crate) fn edits_equivalent(a: &Edit, b: &Edit) -> bool {
    a.start == b.start && a.end == b.end && a.replacement == b.replacement
}

/// 判断两个编辑是否存在区间重叠
pub(crate) fn edits_overlap(a: &Edit, b: &Edit) -> bool {
    if a.start == a.end && b.start == b.end {
        return a.start == b.start;
    }
    if a.start == a.end {
        return b.start <= a.start && a.start < b.end;
    }
    if b.start == b.end {
        return a.start <= b.start && b.start < a.end;
    }
    a.start < b.end && b.start < a.end
}

fn push_edit(edits: &mut Vec<Edit>, edit: Edit) {
    if let Some(last) = edits.last_mut()
        && last.start == edit.start
        && last.end == edit.end
    {
        last.replacement.push_str(&edit.replacement);
        return;
    }
    edits.push(edit);
}

fn slice_by_char(s: &str, start: usize, end: usize) -> &str {
    let byte_start = char_to_byte_index(s, start);
    let byte_end = char_to_byte_index(s, end);
    if byte_start >= s.len() || byte_start >= byte_end {
        return "";
    }
    &s[byte_start..std::cmp::min(byte_end, s.len())]
}

fn char_to_byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(s.len())
}
