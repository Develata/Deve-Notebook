// crates\core\src
//! # 文档状态管理
//!
//! 本模块提供文档状态管理功能：
//!
//! - `reconstruct_content`: 从操作序列重建文档内容
//! - `compute_diff`: 计算两个字符串之间的编辑操作差异
//!
//! 这些函数被后端（用于持久化）和前端（用于同步）共同使用。

use crate::models::{LedgerEntry, Op};
// use anyhow::Result; // Not used currently

/// 将字符索引转换为字节索引
///
/// **参数**:
/// * `s`: 原始字符串
/// * `char_index`: 字符索引（Unicode 码点位置）
///
/// **返回值**:
/// 对应的字节索引。如果字符索引超出范围，返回字符串的字节长度。
fn char_to_byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices()
        .nth(char_index)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(s.len())
}

/// 从操作序列重建文档内容
///
/// **参数**:
/// * `ops`: 按顺序排列的账本条目 (`LedgerEntry`) 列表。
///
/// **逻辑**:
/// 1. 遍历操作列表。
/// 2. 对于 `Insert`，在指定 `pos`（字符索引）插入字符串。
/// 3. 对于 `Delete`，从指定 `pos`（字符索引）删除 `len` 个字符。
///
/// **注意**:
/// - 所有位置都是字符索引（非字节索引），以正确处理 UTF-8 多字节字符（如中文）。
/// - 当前实现假设操作是线性有序的（Phase 0 简化假设）。
/// - 在更复杂的 CRDT 场景中，此处应由 Loro 等库处理。
pub fn reconstruct_content(ops: &[LedgerEntry]) -> String {
    let mut content = String::new();

    for entry in ops {
        match &entry.op {
            Op::Insert { pos, content: text } => {
                // 将字符索引转换为字节索引
                let byte_pos = char_to_byte_index(&content, *pos as usize);
                if byte_pos >= content.len() {
                    content.push_str(text);
                } else {
                    content.insert_str(byte_pos, text);
                }
            }
            Op::Delete { pos, len } => {
                // 将字符索引转换为字节索引
                let byte_start = char_to_byte_index(&content, *pos as usize);
                let byte_end = char_to_byte_index(&content, (*pos + *len) as usize);

                if byte_start < content.len() {
                    let safe_end = std::cmp::min(byte_end, content.len());
                    content.drain(byte_start..safe_end);
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
///
/// **注意**:
/// 所有位置和长度都是字符索引（非字节索引），以正确处理 UTF-8 多字节字符。
pub fn compute_diff(old: &str, new: &str) -> Vec<Op> {
    use dissimilar::Chunk;
    let chunks = dissimilar::diff(old, new);
    let mut ops = Vec::new();
    let mut pos: u32 = 0; // 字符位置

    for chunk in chunks {
        match chunk {
            Chunk::Equal(text) => {
                // 使用字符数量而非字节数量
                pos += text.chars().count() as u32;
            }
            Chunk::Insert(text) => {
                ops.push(Op::Insert {
                    pos,
                    content: text.into(),
                });
                // 使用字符数量而非字节数量
                pos += text.chars().count() as u32;
            }
            Chunk::Delete(text) => {
                ops.push(Op::Delete {
                    pos,
                    // 使用字符数量而非字节数量
                    len: text.chars().count() as u32,
                });
                // 删除内容后，后续字符位置左移，因此 pos 不需要包含被删除的长度
            }
        }
    }
    ops
}
