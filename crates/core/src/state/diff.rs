//! plan_ref:
//!   - 03_rendering#document-authority-bridge

use crate::models::Op;

use super::utf16::{add_utf16_pos, utf16_len};

/// 返回可按顺序应用的 UTF-16 位置 diff。
pub fn compute_diff(old: &str, new: &str) -> Vec<Op> {
    use dissimilar::Chunk;

    let chunks = dissimilar::diff(old, new);
    let mut ops = Vec::new();
    let mut pos: u32 = 0;

    for chunk in chunks {
        match chunk {
            Chunk::Equal(text) => {
                if !add_utf16_pos(&mut pos, text) {
                    return Vec::new();
                }
            }
            Chunk::Insert(text) => {
                ops.push(Op::Insert {
                    pos,
                    content: text.into(),
                });
                if !add_utf16_pos(&mut pos, text) {
                    return Vec::new();
                }
            }
            Chunk::Delete(text) => {
                let len = match utf16_len(text) {
                    Some(v) => v,
                    None => return Vec::new(),
                };
                ops.push(Op::Delete { pos, len });
            }
        }
    }

    ops
}
