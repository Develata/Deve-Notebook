use deve_core::models::Op;
use deve_core::protocol::ConfirmedOp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DeltaChainIssue {
    pub seq: u64,
    pub current_utf16_len: u32,
    pub kind: DeltaChainIssueKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DeltaChainIssueKind {
    BaseContentTooLarge,
    InsertLengthOverflow,
    InsertBeyondEnd { pos: u32 },
    DeleteRangeOverflow { pos: u32, len: u32 },
    DeleteBeyondEnd { pos: u32, len: u32, end: u32 },
}

#[cfg(test)]
pub(super) fn delta_ops_fit(content: &str, delta_ops: &[ConfirmedOp]) -> bool {
    find_delta_chain_issue(content, delta_ops).is_none()
}

pub(super) fn find_delta_chain_issue(
    content: &str,
    delta_ops: &[ConfirmedOp],
) -> Option<DeltaChainIssue> {
    let mut utf16_len = match u32::try_from(content.encode_utf16().count()) {
        Ok(len) => len,
        Err(_) => {
            return Some(DeltaChainIssue {
                seq: delta_ops.first().map(|entry| entry.seq).unwrap_or(0),
                current_utf16_len: u32::MAX,
                kind: DeltaChainIssueKind::BaseContentTooLarge,
            });
        }
    };
    for entry in delta_ops {
        match next_utf16_len(utf16_len, entry) {
            Ok(next_len) => utf16_len = next_len,
            Err(issue) => return Some(issue),
        }
    }
    None
}

fn next_utf16_len(current_len: u32, entry: &ConfirmedOp) -> Result<u32, DeltaChainIssue> {
    match &entry.op {
        Op::Insert { pos, content } => {
            let delta =
                u32::try_from(content.encode_utf16().count()).map_err(|_| DeltaChainIssue {
                    seq: entry.seq,
                    current_utf16_len: current_len,
                    kind: DeltaChainIssueKind::InsertLengthOverflow,
                })?;
            if *pos > current_len {
                return Err(DeltaChainIssue {
                    seq: entry.seq,
                    current_utf16_len: current_len,
                    kind: DeltaChainIssueKind::InsertBeyondEnd { pos: *pos },
                });
            }
            current_len.checked_add(delta).ok_or(DeltaChainIssue {
                seq: entry.seq,
                current_utf16_len: current_len,
                kind: DeltaChainIssueKind::InsertLengthOverflow,
            })
        }
        Op::Delete { pos, len } => {
            let end = pos.checked_add(*len).ok_or(DeltaChainIssue {
                seq: entry.seq,
                current_utf16_len: current_len,
                kind: DeltaChainIssueKind::DeleteRangeOverflow {
                    pos: *pos,
                    len: *len,
                },
            })?;
            if end > current_len {
                return Err(DeltaChainIssue {
                    seq: entry.seq,
                    current_utf16_len: current_len,
                    kind: DeltaChainIssueKind::DeleteBeyondEnd {
                        pos: *pos,
                        len: *len,
                        end,
                    },
                });
            }
            Ok(current_len - *len)
        }
    }
}

pub(super) fn issue_summary(issue: DeltaChainIssue) -> String {
    match issue.kind {
        DeltaChainIssueKind::BaseContentTooLarge => {
            format!("base content UTF-16 length overflow at seq {}", issue.seq)
        }
        DeltaChainIssueKind::InsertLengthOverflow => format!(
            "insert length overflow at seq {} (current_utf16_len={})",
            issue.seq, issue.current_utf16_len
        ),
        DeltaChainIssueKind::InsertBeyondEnd { pos } => format!(
            "insert beyond end at seq {}: pos={} current_utf16_len={}",
            issue.seq, pos, issue.current_utf16_len
        ),
        DeltaChainIssueKind::DeleteRangeOverflow { pos, len } => format!(
            "delete range overflow at seq {}: pos={} len={} current_utf16_len={}",
            issue.seq, pos, len, issue.current_utf16_len
        ),
        DeltaChainIssueKind::DeleteBeyondEnd { pos, len, end } => format!(
            "delete beyond end at seq {}: pos={} len={} end={} current_utf16_len={}",
            issue.seq, pos, len, end, issue.current_utf16_len
        ),
    }
}
