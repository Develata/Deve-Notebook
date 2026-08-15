// crates\core\src
//! # 文档状态管理
//! plan_ref:
//!   - 10_rendering#document-authority-bridge
//!   - 10_rendering#large-document-runtime
//!
//! 本模块提供文档状态管理功能：
//!
//! - `reconstruct_content`: 从操作序列重建文档内容
//! - `compute_diff`: 计算两个字符串之间的编辑操作差异
//!
//! 这些函数被后端（用于持久化）和前端（用于同步）共同使用。

use crate::models::{LedgerEntry, Op};
use rope_utf16::Utf16IndexCache;
use ropey::Rope;

mod diff;
mod rope_utf16;
mod utf16;
mod validate;

pub use diff::{ComputeDiffError, compute_diff};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use validate::ContentOpValidator;
pub use validate::{InvalidContentOp, describe_invalid_content_op, find_invalid_content_op};

/// 从操作序列重建文档内容
///
/// **参数**:
/// * `ops`: 按顺序排列的账本条目 (`LedgerEntry`) 列表。
///
/// **逻辑**:
/// 1. 遍历操作列表。
/// 2. 对于 `Insert`，在指定 `pos`（UTF-16 索引）插入字符串。
/// 3. 对于 `Delete`，从指定 `pos`（UTF-16 索引）删除 `len` 个 UTF-16 code unit。
///
/// **注意**:
/// - 所有位置都是 UTF-16 code unit 索引（非字节索引），与 JS/CodeMirror 一致。
/// - 当前实现假设操作是线性有序的（Phase 0 简化假设）。
/// - 在更复杂的 CRDT 场景中，此处应由 Loro 等库处理。
pub fn reconstruct_content(ops: &[LedgerEntry]) -> String {
    reconstruct_content_until(ops, || false).expect("non-cancellable reconstruction completes")
}

/// 重建内容，并允许后台 projection/prewarm 在 runtime shutdown 时协作取消。
///
/// 返回 `None` 时调用方不得保存部分结果或推进任何 authority waterline。
pub fn reconstruct_content_until(
    ops: &[LedgerEntry],
    mut cancelled: impl FnMut() -> bool,
) -> Option<String> {
    let mut content = Rope::new();
    let mut total_utf16: u32 = 0;
    let mut cache = Utf16IndexCache::new(adaptive_step(total_utf16));
    let mut op_count = 0u32;

    for entry in ops {
        if cancelled() {
            return None;
        }
        op_count = op_count.wrapping_add(1);
        let Some(op) = entry.content_op() else {
            continue;
        };
        match op {
            Op::Insert { pos, content: text } => {
                let char_idx = cache.locate(&content, *pos);
                let utf16_delta = text.encode_utf16().count() as u32;
                let char_delta = text.chars().count();
                content.insert(char_idx, text);
                total_utf16 = total_utf16.saturating_add(utf16_delta);
                let next_step = adaptive_step(total_utf16);
                if cache.update_after_insert(*pos, utf16_delta, char_delta)
                    || cache.step() != next_step
                {
                    cache = Utf16IndexCache::build(&content, next_step);
                }
            }
            Op::Delete { pos, len } => {
                let end_pos = pos.checked_add(*len).unwrap_or(u32::MAX);
                let start_idx = cache.locate(&content, *pos);
                let end_idx = cache.locate(&content, end_pos);
                if end_idx > start_idx {
                    let removed_slice = content.slice(start_idx..end_idx);
                    let mut removed_utf16 = 0u32;
                    let mut removed_chars = 0usize;
                    for ch in removed_slice.chars() {
                        removed_utf16 += ch.len_utf16() as u32;
                        removed_chars += 1;
                    }
                    content.remove(start_idx..end_idx);
                    total_utf16 = total_utf16.saturating_sub(removed_utf16);
                    let next_step = adaptive_step(total_utf16);
                    if cache.update_after_delete(*pos, removed_utf16, removed_utf16, removed_chars)
                        || cache.step() != next_step
                    {
                        cache = Utf16IndexCache::build(&content, next_step);
                    }
                }
            }
        }
        if op_count.is_multiple_of(256) && !cache.validate_sample(&content) {
            cache = Utf16IndexCache::build(&content, adaptive_step(total_utf16));
        }
    }

    if cancelled() {
        return None;
    }
    Some(content.to_string())
}

/// 将一组 UTF-16 索引的内容操作应用到已有文本。
///
/// 返回 `None` 表示操作无法按当前文本边界应用；调用方必须 fail closed，
/// 不能在本地投影未更新时推进版本。
pub fn try_apply_content_ops(base: &str, ops: &[Op]) -> Option<String> {
    let mut content = Rope::from_str(base);
    let mut total_utf16 = utf16::utf16_len(base)?;
    let mut cache = Utf16IndexCache::build(&content, adaptive_step(total_utf16));

    for op in ops {
        match op {
            Op::Insert { pos, content: text } => {
                if *pos > total_utf16 {
                    return None;
                }
                let delta = utf16::utf16_len(text)?;
                let next_len = total_utf16.checked_add(delta)?;
                let char_idx = cache.locate(&content, *pos);
                let char_delta = text.chars().count();
                content.insert(char_idx, text);
                total_utf16 = next_len;
                let next_step = adaptive_step(total_utf16);
                if cache.update_after_insert(*pos, delta, char_delta) || cache.step() != next_step {
                    cache = Utf16IndexCache::build(&content, next_step);
                }
            }
            Op::Delete { pos, len } => {
                let end_pos = pos.checked_add(*len)?;
                if *pos > total_utf16 || end_pos > total_utf16 {
                    return None;
                }
                let start_idx = cache.locate(&content, *pos);
                let end_idx = cache.locate(&content, end_pos);
                if *len > 0 && end_idx <= start_idx {
                    return None;
                }
                if end_idx > start_idx {
                    let removed_slice = content.slice(start_idx..end_idx);
                    let mut removed_utf16 = 0u32;
                    let mut removed_chars = 0usize;
                    for ch in removed_slice.chars() {
                        removed_utf16 += ch.len_utf16() as u32;
                        removed_chars += 1;
                    }
                    if removed_utf16 != *len {
                        return None;
                    }
                    content.remove(start_idx..end_idx);
                    total_utf16 = total_utf16.checked_sub(removed_utf16)?;
                    let next_step = adaptive_step(total_utf16);
                    if cache.update_after_delete(*pos, removed_utf16, removed_utf16, removed_chars)
                        || cache.step() != next_step
                    {
                        cache = Utf16IndexCache::build(&content, next_step);
                    }
                }
            }
        }
    }

    Some(content.to_string())
}

fn adaptive_step(total_utf16: u32) -> u32 {
    let step = total_utf16 / 64;
    step.clamp(64, 1024)
}

#[cfg(test)]
mod tests;
