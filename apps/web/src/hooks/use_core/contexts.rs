//! plan_ref:
//!   - 07_network#web-ws-runtime
//!   - 04_repository#repo-scope-runtime
//!
//! 子上下文导出入口。
#![allow(dead_code)]

mod branch;
mod chat;
mod dashboard;
mod doc;
mod editor;
mod source_control;
mod sync;

pub use branch::{BranchContext, RepoRemoveRequest, RepoRenameRequest, RepoSwitchRequest};
pub use chat::ChatContext;
pub use dashboard::{DashboardContext, SystemMetricsData};
pub use doc::DocContext;
pub use editor::EditorContext;
pub use source_control::SourceControlContext;
pub use sync::SyncMergeContext;
