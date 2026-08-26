//! plan_ref:
//!   - 04_repository#tree-projection-contract
//!   - 05_diff_logic#source-control-runtime
//!
//! Typed errors for commit-diff projection and content reconstruction.

use crate::models::{DocId, NodeId};

pub(crate) type CommitDiffResult<T> = std::result::Result<T, CommitDiffError>;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum CommitDiffError {
    #[error("failed to {action} Deve commit table: {message}")]
    CommitTable {
        action: &'static str,
        message: String,
    },
    #[error("failed to load Deve commit {commit_id}: {message}")]
    CommitLoad { commit_id: String, message: String },
    #[error("Commit not found: {commit_id}")]
    CommitNotFound { commit_id: String },
    #[error("failed to decode Deve commit {commit_id}: {message}")]
    CommitDecode { commit_id: String, message: String },
    #[error("failed to read ledger ops in range {start}..{end}: {message}")]
    LedgerRange {
        start: u64,
        end: u64,
        message: String,
    },
    #[error("Commit diff invalid order: base seq {seq_a} must be older than target seq {seq_b}")]
    InvalidOrder { seq_a: u64, seq_b: u64 },
    #[error("failed to reconstruct doc {doc_id} at seq {max_seq}: {message}")]
    ContentLoad {
        doc_id: DocId,
        max_seq: u64,
        message: String,
    },
    #[error("Commit diff lost projected path for doc {doc_id} between seq {seq_a} and {seq_b}")]
    LostProjectedPath {
        doc_id: DocId,
        seq_a: u64,
        seq_b: u64,
    },
    #[error("Commit diff structure maps doc {doc_id} to multiple live paths: {existing}, {path}")]
    MultipleLivePaths {
        doc_id: DocId,
        existing: String,
        path: String,
    },
    #[error("Commit diff structure rename references missing node {node_id}")]
    RenameMissingNode { node_id: NodeId },
    #[error("Commit diff structure move references missing node {node_id}")]
    MoveMissingNode { node_id: NodeId },
    #[error("Commit diff structure contains duplicate live node {node_id}")]
    DuplicateStructureNode { node_id: NodeId },
    #[error("Commit diff structure file node {node_id} does not match doc {doc_id}")]
    FileNodeDocMismatch { node_id: NodeId, doc_id: DocId },
    #[error("Commit diff structure parent {parent_id} of node {node_id} is not a directory")]
    StructureParentNotDirectory { node_id: NodeId, parent_id: NodeId },
    #[error("Commit diff structure path collision at {path} between {existing} and {node_id}")]
    StructurePathCollision {
        path: String,
        existing: NodeId,
        node_id: NodeId,
    },
    #[error("Commit diff structure contains cycle at node {node_id}")]
    StructureCycle { node_id: NodeId },
    #[error("Commit diff structure references missing node {node_id}")]
    StructureMissingNode { node_id: NodeId },
    #[error("Commit file diff target no longer matches the requested comparison")]
    TargetMismatch,
}
