//! plan_ref:
//!   - 10_rendering#large-document-runtime
//!   - 10_rendering#document-authority-bridge
//!
use crate::editor::ffi::applyRemoteOpsBatch;
use crate::runtime::domain::EditorSyncFailureCode;
use deve_core::models::Op;
use deve_core::protocol::ConfirmedOp;

pub(super) fn build_history_replay_ops(
    history: &[ConfirmedOp],
    buffered: &mut [ConfirmedOp],
    base_version: u64,
    client_id: Option<u64>,
    pending_ops: Vec<Op>,
) -> Result<Vec<Op>, EditorSyncFailureCode> {
    if history.windows(2).any(|pair| pair[0].seq >= pair[1].seq) {
        return Err(EditorSyncFailureCode::HistoryReplay);
    }

    buffered.sort_by_key(|entry| entry.seq);
    if buffered
        .windows(2)
        .any(|pair| pair[0].seq == pair[1].seq && pair[0] != pair[1])
    {
        return Err(EditorSyncFailureCode::LiveReplay);
    }
    let history_waterline = history
        .last()
        .map(|entry| entry.seq)
        .unwrap_or(base_version)
        .max(base_version);
    for entry in buffered
        .iter()
        .filter(|entry| entry.seq > base_version && entry.seq <= history_waterline)
    {
        match history.binary_search_by_key(&entry.seq, |candidate| candidate.seq) {
            Ok(index) if history[index] == *entry => {}
            _ => return Err(EditorSyncFailureCode::LiveReplay),
        }
    }

    let mut replay_ops = history
        .iter()
        .filter(|entry| entry.seq > base_version)
        .map(|entry| entry.op.clone())
        .collect::<Vec<_>>();
    let mut last_buffered_seq = None;
    for entry in buffered.iter().filter(|entry| {
        entry.seq > history_waterline
            && entry
                .origin
                .is_none_or(|origin| Some(origin.client_id) != client_id)
    }) {
        if last_buffered_seq == Some(entry.seq) {
            continue;
        }
        last_buffered_seq = Some(entry.seq);
        replay_ops.push(entry.op.clone());
    }
    replay_ops.extend(pending_ops);
    Ok(replay_ops)
}

pub(super) fn merge_history_tail(
    history: &[ConfirmedOp],
    mut live_history: Vec<(u64, deve_core::models::Op)>,
) -> Vec<(u64, deve_core::models::Op)> {
    let mut last_seq = live_history.last().map(|(seq, _)| *seq).unwrap_or(0);
    for entry in history {
        if entry.seq > last_seq {
            live_history.push((entry.seq, entry.op.clone()));
            last_seq = entry.seq;
        }
    }
    live_history
}

pub(super) fn apply_replay_ops_atomically(ops: &[Op]) -> Result<(), EditorSyncFailureCode> {
    if ops.is_empty() {
        return Ok(());
    }
    match serde_json::to_string(&ops) {
        Ok(json) => {
            if !applyRemoteOpsBatch(&json) {
                leptos::logging::warn!("History/pending/live replay batch failed");
                return Err(EditorSyncFailureCode::HistoryReplay);
            }
        }
        Err(err) => {
            leptos::logging::warn!("History/pending/live replay serialization failed: {err}");
            return Err(EditorSyncFailureCode::HistoryReplay);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::build_history_replay_ops;
    use crate::runtime::domain::EditorSyncFailureCode;
    use deve_core::models::Op;
    use deve_core::protocol::{ClientOrigin, ConfirmedOp};

    fn insert(seq: u64, pos: u32, content: &str) -> ConfirmedOp {
        ConfirmedOp::new(
            seq,
            Op::Insert {
                pos,
                content: content.into(),
            },
            None,
        )
    }

    #[test]
    fn history_replay_includes_authoritative_tail_before_pending_overlay() {
        let history = vec![insert(2, 1, "h")];
        let mut buffered = Vec::new();
        let pending = vec![Op::Insert {
            pos: 2,
            content: "p".into(),
        }];

        let replay = build_history_replay_ops(&history, &mut buffered, 1, None, pending)
            .expect("history replay plan");

        assert_eq!(replay.len(), 2);
        assert_eq!(replay[0], history[0].op);
        assert!(matches!(&replay[1], Op::Insert { content, .. } if content == "p"));
    }

    #[test]
    fn history_replay_deduplicates_identical_buffered_entries() {
        let entry = insert(3, 1, "x");
        let mut buffered = vec![entry.clone(), entry];

        let replay = build_history_replay_ops(&[], &mut buffered, 2, None, Vec::new())
            .expect("identical duplicates are idempotent");

        assert_eq!(
            buffered.len(),
            2,
            "planning must not mutate the failure buffer"
        );
        assert_eq!(replay.len(), 1);
    }

    #[test]
    fn history_replay_rejects_conflicting_buffered_sequence_without_dropping_entries() {
        let mut buffered = vec![insert(3, 1, "x"), insert(3, 1, "y")];

        let error = build_history_replay_ops(&[], &mut buffered, 2, None, Vec::new())
            .expect_err("conflicting duplicate must fail closed");

        assert_eq!(error, EditorSyncFailureCode::LiveReplay);
        assert_eq!(buffered.len(), 2);
    }

    #[test]
    fn history_replay_skips_buffered_echo_already_present_as_pending() {
        let mut buffered = vec![ConfirmedOp::new(
            3,
            Op::Insert {
                pos: 1,
                content: "x".into(),
            },
            Some(ClientOrigin {
                client_id: 7,
                client_op_id: 9,
            }),
        )];
        let pending = vec![buffered[0].op.clone()];

        let replay = build_history_replay_ops(&[], &mut buffered, 2, Some(7), pending)
            .expect("echo is represented by pending overlay");

        assert_eq!(replay.len(), 1);
    }
}
