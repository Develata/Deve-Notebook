//! plan_ref:
//!   - 05_network#web-ws-runtime
//!   - 06_repository#repo-scope-runtime
//!
//! 子上下文导出入口。
#![allow(dead_code)]

#[path = "contexts_branch.rs"]
mod branch;
#[path = "contexts_chat.rs"]
mod chat;
#[path = "contexts_dashboard.rs"]
mod dashboard;
#[path = "contexts_doc.rs"]
mod doc;
#[path = "contexts_editor.rs"]
mod editor;
#[path = "contexts_source_control.rs"]
mod source_control;
#[path = "contexts_sync.rs"]
mod sync;

pub use branch::BranchContext;
pub use chat::ChatContext;
pub use dashboard::{DashboardContext, SystemMetricsData};
pub use doc::DocContext;
pub use editor::EditorContext;
pub use source_control::SourceControlContext;
pub use sync::SyncMergeContext;
