//! plan_ref:
//!   - 10_rendering#document-authority-bridge

use crate::models::Op;

use super::utf16::utf16_len;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ComputeDiffError {
    #[error("UTF-16 diff position exceeds the supported u32 range")]
    PositionOverflow,
}

/// 返回可按顺序应用的 UTF-16 位置 diff。
pub fn compute_diff(old: &str, new: &str) -> Result<Vec<Op>, ComputeDiffError> {
    use dissimilar::Chunk;

    let chunks = dissimilar::diff(old, new);
    let mut ops = Vec::new();
    let mut pos: u32 = 0;

    for chunk in chunks {
        match chunk {
            Chunk::Equal(text) => {
                pos = advance_utf16_position(pos, utf16_len(text))?;
            }
            Chunk::Insert(text) => {
                ops.push(Op::Insert {
                    pos,
                    content: text.into(),
                });
                pos = advance_utf16_position(pos, utf16_len(text))?;
            }
            Chunk::Delete(text) => {
                let len = utf16_len(text).ok_or(ComputeDiffError::PositionOverflow)?;
                ops.push(Op::Delete { pos, len });
            }
        }
    }

    Ok(ops)
}

fn advance_utf16_position(position: u32, delta: Option<u32>) -> Result<u32, ComputeDiffError> {
    position
        .checked_add(delta.ok_or(ComputeDiffError::PositionOverflow)?)
        .ok_or(ComputeDiffError::PositionOverflow)
}

#[cfg(test)]
mod tests {
    use super::{ComputeDiffError, advance_utf16_position};

    #[test]
    fn compute_diff_position_overflow_fails_closed() {
        assert_eq!(
            advance_utf16_position(u32::MAX, Some(1)),
            Err(ComputeDiffError::PositionOverflow)
        );
        assert_eq!(
            advance_utf16_position(0, None),
            Err(ComputeDiffError::PositionOverflow)
        );
    }
}
