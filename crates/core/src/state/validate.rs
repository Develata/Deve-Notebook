//! plan_ref:
//!   - 10_rendering#document-authority-bridge

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

/// 增量校验 content ops 的 UTF-16 长度边界。
///
/// append validation 使用它在扫描既有历史后直接推进候选 entry，
/// 避免 clone 候选 entry 并第二次全量重扫。
#[derive(Default)]
pub(crate) struct ContentOpValidator {
    current_utf16_len: u32,
}

impl ContentOpValidator {
    pub fn push_entry(&mut self, entry: &LedgerEntry) -> Option<InvalidContentOp> {
        let op = entry.content_op()?;

        match op {
            Op::Insert { pos, content } => {
                if *pos > self.current_utf16_len {
                    return Some(InvalidContentOp::InsertBeyondEnd {
                        seq: entry.seq,
                        pos: *pos,
                        current_utf16_len: self.current_utf16_len,
                    });
                }
                let Some(delta) = utf16_len(content) else {
                    return Some(InvalidContentOp::LengthOverflow { seq: entry.seq });
                };
                let Some(next_len) = self.current_utf16_len.checked_add(delta) else {
                    return Some(InvalidContentOp::LengthOverflow { seq: entry.seq });
                };
                self.current_utf16_len = next_len;
            }
            Op::Delete { pos, len } => {
                let Some(end) = pos.checked_add(*len) else {
                    return Some(InvalidContentOp::LengthOverflow { seq: entry.seq });
                };
                if *pos > self.current_utf16_len || end > self.current_utf16_len {
                    return Some(InvalidContentOp::DeleteBeyondEnd {
                        seq: entry.seq,
                        pos: *pos,
                        len: *len,
                        current_utf16_len: self.current_utf16_len,
                    });
                }
                self.current_utf16_len -= *len;
            }
        }

        None
    }
}

pub fn find_invalid_content_op(ops: &[LedgerEntry]) -> Option<InvalidContentOp> {
    let mut validator = ContentOpValidator::default();

    for entry in ops {
        if let Some(issue) = validator.push_entry(entry) {
            return Some(issue);
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
