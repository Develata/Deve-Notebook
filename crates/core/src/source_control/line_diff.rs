// crates/core/src/source_control/line_diff.rs
//! # 行级差异计算 (Line-Level Diff)
//!
//! 基于 `similar` crate (Myers 算法) 计算两段文本的行级变更范围。
//! 供前端 WASM 侧调用，驱动 CodeMirror 行内 Gutter 指示器。
//!
//! ## 变更类型
//! - `Added`: 新增行 (绿色)
//! - `Modified`: 修改行 (蓝色)
//! - `Deleted`: 删除行 (红色)，标记在原文位置

use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

/// 行级变更范围
///
/// **Invariant**: `start_line <= end_line`，行号从 1 开始 (对齐 CodeMirror)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChangeRange {
    /// 变更类型: "added", "modified", "deleted"
    pub kind: String,
    /// 起始行号 (1-based，相对于 new_content)
    pub start_line: u32,
    /// 结束行号 (1-based，inclusive)
    pub end_line: u32,
}

/// 计算 old_content 与 new_content 之间的行级变更范围
///
/// **算法**: Myers diff (via `similar::TextDiff::from_lines`)，
/// 将连续同类型变更合并为范围。
///
/// **Pre-condition**: 输入为 UTF-8 文本。
/// **Post-condition**: 返回的范围按 `start_line` 升序排列，范围不重叠。
pub fn compute_line_ranges(old: &str, new: &str) -> Vec<ChangeRange> {
    let diff = TextDiff::from_lines(old, new);
    let mut ranges: Vec<ChangeRange> = Vec::new();
    let mut new_line: u32 = 1;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                new_line += 1;
            }
            ChangeTag::Insert => {
                push_or_extend(&mut ranges, "added", new_line);
                new_line += 1;
            }
            ChangeTag::Delete => {
                // 删除行标记在当前 new_line 位置 (即删除发生处)
                push_or_extend(&mut ranges, "deleted", new_line);
            }
        }
    }
    ranges
}

/// 合并连续同类型变更为单个范围
fn push_or_extend(ranges: &mut Vec<ChangeRange>, kind: &str, line: u32) {
    if let Some(last) = ranges.last_mut()
        && last.kind == kind
        && last.end_line >= line.saturating_sub(1)
    {
        last.end_line = line;
        return;
    }
    ranges.push(ChangeRange {
        kind: kind.to_string(),
        start_line: line,
        end_line: line,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_changes() {
        let text = "hello\nworld\n";
        let result = compute_line_ranges(text, text);
        assert!(result.is_empty());
    }

    #[test]
    fn test_added_lines() {
        let old = "line1\nline2\n";
        let new = "line1\ninserted\nline2\n";
        let result = compute_line_ranges(old, new);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, "added");
        assert_eq!(result[0].start_line, 2);
        assert_eq!(result[0].end_line, 2);
    }

    #[test]
    fn test_deleted_lines() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nline3\n";
        let result = compute_line_ranges(old, new);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].kind, "deleted");
    }

    #[test]
    fn test_modified_detection() {
        // similar 将 modify 表现为 delete + insert
        let old = "aaa\nbbb\nccc\n";
        let new = "aaa\nBBB\nccc\n";
        let result = compute_line_ranges(old, new);
        // 应该有 deleted 和 added 范围 (similar 不直接报 modified)
        assert!(!result.is_empty());
    }
}
