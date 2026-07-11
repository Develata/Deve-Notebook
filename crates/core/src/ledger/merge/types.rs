// crates\core\src\ledger\merge\types.rs
//! plan_ref:
//!   - 03_storage/authority#facts-partition
//!   - 10_rendering#document-authority-bridge
//!
// ---------------------------------------------------------------
// 模块：三路合并类型定义
// 作用：为合并引擎提供统一的数据结构
// 功能：合并结果与冲突片段的结构化描述
// ---------------------------------------------------------------

use crate::models::{DocId, MergeResolution, PeerFactSeq, PeerId};
use serde::{Deserialize, Serialize};

pub use crate::protocol::ConflictHunk;

/// 合并操作结果
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MergeResult {
    /// 自动合并成功
    Success(String),
    /// 发生冲突
    Conflict {
        base: String,
        local: String,
        remote: String,
        conflicts: Vec<ConflictHunk>,
    },
}

/// Durable local pointer to the source state consumed by the latest merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeBaseCheckpoint {
    pub source_peer_id: PeerId,
    pub doc_id: DocId,
    pub local_anchor_peer_seq: PeerFactSeq,
    pub source_peer_seq: PeerFactSeq,
    pub source_state_hash: [u8; 32],
    pub result_hash: [u8; 32],
    pub anchor_global_seq: u64,
}

/// Evidence captured while evaluating a merge and rechecked by the atomic writer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergePreflight {
    pub(crate) source_peer_id: PeerId,
    pub(crate) repo_id: crate::models::RepoId,
    pub(crate) doc_id: DocId,
    pub(crate) expected_local_waterline: PeerFactSeq,
    pub(crate) expected_source_waterline: PeerFactSeq,
    pub(crate) expected_checkpoint_anchor_global_seq: Option<u64>,
    pub(crate) local_content: String,
    pub(crate) source_content: String,
    pub(crate) source_state_hash: [u8; 32],
    pub(crate) establish_equal: bool,
    pub(crate) automatic_result: Option<String>,
}

impl MergePreflight {
    pub fn doc_id(&self) -> DocId {
        self.doc_id
    }

    pub fn establishes_equal_baseline(&self) -> bool {
        self.establish_equal
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MergeEvaluation {
    pub result: MergeResult,
    pub preflight: MergePreflight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MergeCommitOutcome {
    pub content_changed: bool,
    pub anchor_global_seq: u64,
    pub anchor_peer_seq: PeerFactSeq,
    pub resolution: MergeResolution,
}
