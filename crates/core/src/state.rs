//! # 文档状态管理
//!
//! 本模块提供文档状态管理功能：
//!
//! - `reconstruct_content`: 从操作序列重建文档内容
//! - `compute_diff`: 计算两个字符串之间的编辑操作差异
//!
//! 这些函数被后端（用于持久化）和前端（用于同步）共同使用。

use crate::models::{Op, LedgerEntry};
// use anyhow::Result; // Not used currently

/// 从操作序列重建文档内容
///
/// **参数**:
/// * `ops`: 按顺序排列的账本条目 (`LedgerEntry`) 列表。
///
/// **逻辑**:
/// 1. 遍历操作列表。
/// 2. 对于 `Insert`，在指定 `pos` 插入字符串。
/// 3. 对于 `Delete`，从指定 `pos` 删除 `len` 个字符。
///
/// **注意**: 
/// 当前实现假设操作是线性有序的（Phase 0 简化假设）。
/// 在更复杂的 CRDT 场景中，此处应由 Loro 等库处理。
pub fn reconstruct_content(ops: &[LedgerEntry]) -> String {
    let mut content = String::new();
    
    for entry in ops {
        match &entry.op {
            Op::Insert { pos, content: text } => {
                if *pos >= content.len() {
                    content.push_str(text);
                } else {
                    content.insert_str(*pos, text);
                }
            }
            Op::Delete { pos, len } => {
                if *pos < content.len() {
                    let end = std::cmp::min(pos + len, content.len());
                    content.drain(*pos..end);
                }
            }
        }
    }
    
    content
}

/// 计算两个字符串之间的编辑操作差异
///
/// **参数**:
/// * `old`: 旧文本内容。
/// * `new`: 新文本内容。
///
/// **返回值**:
/// 返回一系列原子操作 (`Op::Insert` 或 `Op::Delete`)，
/// 按顺序应用这些操作可以将 `old` 转换为 `new`。
///
/// **实现**:
/// 使用 `dissimilar` 库计算 diff，然后转换为我们的 `Op` 枚举。
pub fn compute_diff(old: &str, new: &str) -> Vec<Op> {
    use dissimilar::Chunk;
    let chunks = dissimilar::diff(old, new);
    let mut ops = Vec::new();
    let mut pos = 0;
    
    for chunk in chunks {
        match chunk {
            Chunk::Equal(text) => {
                pos += text.len();
            }
            Chunk::Insert(text) => {
                ops.push(Op::Insert {
                    pos,
                    content: text.to_string(),
                });
                pos += text.len();
            }
            Chunk::Delete(text) => {
                ops.push(Op::Delete {
                    pos,
                    len: text.len(),
                });
                // 删除内容后，后续字符位置左移，因此 pos 不需要包含被删除的长度
            }
        }
    }
    ops
}
