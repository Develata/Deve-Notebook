//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!
//! Source-control operation context for error mapping.

pub enum ScOp {
    ListPending,
    ListChanges,
    StagePending(String),
    DiscardPending(String),
    Unstage(String),
    DiffDoc(String),
    CommitHistory,
    CommitDiff(String),
    Commit,
    ApplyExternalChanges,
    RemoteProjection,
}
