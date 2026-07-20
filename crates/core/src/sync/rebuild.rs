//! plan_ref:
//!   - 03_storage/projection#projection-contract

use crate::ledger::RepoManager;
use crate::ledger::ops;
use crate::models::{DocId, LedgerEntry, Op, PeerId};
use anyhow::Result;

/// 重建结果：用于调用方决定是否保存快照、上报版本等。
pub struct RebuildResult {
    pub content: String,
    pub base_seq: u64,
    pub max_seq: u64,
}

/// 从“最新快照 + 增量操作”重建本地文档内容。
///
/// Invariants:
/// - 对同一 `doc_id`，操作必须按 `seq` 升序应用。
/// - 若存在快照，其 `seq` 必须小于等于随后应用的所有操作 `seq`。
///
/// Pre-conditions:
/// - 快照内容（若存在）应对应快照序列号的真实状态。
///
/// Post-conditions:
/// - 返回内容等价于从空状态依次应用该文档全部操作后的结果。
pub fn rebuild_local_doc_in_repo(
    repo: &RepoManager,
    repo_name: &str,
    doc_id: DocId,
) -> Result<RebuildResult> {
    let (base_seq, base_content) =
        match repo.load_latest_snapshot_in_local_repo(repo_name, doc_id)? {
            Some((seq, content)) => (seq, content),
            None => (0, String::new()),
        };

    let delta_entries = repo.run_on_local_repo(repo_name, |db| {
        ops::get_ops_from_db_after(db, doc_id, base_seq)
    })?;
    rebuild_from_snapshot_and_delta(doc_id, base_seq, base_content, delta_entries)
}

fn rebuild_from_snapshot_and_delta(
    doc_id: DocId,
    base_seq: u64,
    base_content: String,
    delta_entries: Vec<(u64, LedgerEntry)>,
) -> Result<RebuildResult> {
    let mut entries = Vec::new();
    if !base_content.is_empty() {
        entries.push(LedgerEntry::new_content(
            doc_id,
            Op::Insert {
                pos: 0,
                content: base_content.into(),
            },
            0,
            PeerId::new("snapshot"),
            base_seq,
            None,
            None,
        ));
    }

    let mut max_seq = base_seq;
    for (seq, entry) in delta_entries {
        max_seq = max_seq.max(seq);
        entries.push(entry);
    }

    let content = crate::state::reconstruct_content(&entries);
    Ok(RebuildResult {
        content,
        base_seq,
        max_seq,
    })
}
