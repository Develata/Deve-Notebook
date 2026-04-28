//! plan_ref:
//!   - 03_rendering#document-authority-bridge

use crate::models::{LedgerEntry, Op};

use super::utf16::utf16_len;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidContentOp {
    LengthOverflow {
        seq: u64,
    },
    InsertBeyondEnd {
        seq: u64,
        pos: u32,
        current_utf16_len: u32,
    },
    DeleteBeyondEnd {
        seq: u64,
        pos: u32,
        len: u32,
        current_utf16_len: u32,
    },
}

pub fn find_invalid_content_op(ops: &[LedgerEntry]) -> Option<InvalidContentOp> {
    let mut current_utf16_len = 0u32;

    for entry in ops {
        let Some(op) = entry.content_op() else {
            continue;
        };

        match op {
            Op::Insert { pos, content } => {
                if *pos > current_utf16_len {
                    return Some(InvalidContentOp::InsertBeyondEnd {
                        seq: entry.seq,
                        pos: *pos,
                        current_utf16_len,
                    });
                }
                let Some(delta) = utf16_len(content) else {
                    return Some(InvalidContentOp::LengthOverflow { seq: entry.seq });
                };
                let Some(next_len) = current_utf16_len.checked_add(delta) else {
                    return Some(InvalidContentOp::LengthOverflow { seq: entry.seq });
                };
                current_utf16_len = next_len;
            }
            Op::Delete { pos, len } => {
                let Some(end) = pos.checked_add(*len) else {
                    return Some(InvalidContentOp::LengthOverflow { seq: entry.seq });
                };
                if *pos > current_utf16_len || end > current_utf16_len {
                    return Some(InvalidContentOp::DeleteBeyondEnd {
                        seq: entry.seq,
                        pos: *pos,
                        len: *len,
                        current_utf16_len,
                    });
                }
                current_utf16_len -= *len;
            }
        }
    }

    None
}

pub fn describe_invalid_content_op(issue: &InvalidContentOp) -> String {
    match issue {
        InvalidContentOp::LengthOverflow { seq } => {
            format!("utf16 length overflow at seq {}", seq)
        }
        InvalidContentOp::InsertBeyondEnd {
            seq,
            pos,
            current_utf16_len,
        } => format!(
            "insert beyond end at seq {}: pos={} current_utf16_len={}",
            seq, pos, current_utf16_len
        ),
        InvalidContentOp::DeleteBeyondEnd {
            seq,
            pos,
            len,
            current_utf16_len,
        } => format!(
            "delete beyond end at seq {}: pos={} len={} current_utf16_len={}",
            seq, pos, len, current_utf16_len
        ),
    }
}
